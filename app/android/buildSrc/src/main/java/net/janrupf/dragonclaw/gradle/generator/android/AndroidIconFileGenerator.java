package net.janrupf.dragonclaw.gradle.generator.android;

import com.android.ide.common.vectordrawable.Svg2Vector;
import com.android.utils.PositionXmlParser;
import com.android.utils.XmlUtils;
import net.janrupf.dragonclaw.gradle.generator.IconFileGenerator;
import net.janrupf.dragonclaw.gradle.meta.android.AndroidIconTargetOptions;
import org.gradle.api.logging.Logger;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;

import javax.xml.xpath.XPathExpressionException;
import javax.xml.xpath.XPathFactory;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Converts an SVG icon to an Android icon.
 */
public class AndroidIconFileGenerator extends IconFileGenerator {
    private final XPathFactory xPathFactory;

    private final AndroidIconTargetOptions options;
    private final File drawableDirectory;
    private final File drawableV26Directory;

    public AndroidIconFileGenerator(
            File metaFile,
            File iconFile,
            File outputDirectory,
            AndroidIconTargetOptions options
    ) {
        super(metaFile, iconFile, outputDirectory);
        this.xPathFactory = XPathFactory.newInstance();

        this.options = options;
        this.drawableDirectory = new File(outputDirectory, "drawable");
        this.drawableV26Directory = new File(outputDirectory, "drawable-v26");
    }


    @Override
    public void generate(Logger logger) throws Exception {
        // Make sure the output directories exist
        if (!drawableDirectory.mkdirs() && !drawableDirectory.isDirectory()) {
            throw new IOException("Failed to create directory " + drawableDirectory.getAbsolutePath());
        }

        if (!drawableV26Directory.mkdirs() && !drawableV26Directory.isDirectory()) {
            throw new IOException("Failed to create directory " + drawableV26Directory.getAbsolutePath());
        }

        String resourceName = options.getResourceName();

        // Generate a android vector drawable
        File outputFile = new File(drawableDirectory, resourceName + ".xml");
        convertSvgToAndroid(logger, getIconFile().toPath(), outputFile);

        // Generate adaptive icons
        String svgContent = Files.readString(getIconFile().toPath());
        Document document = PositionXmlParser.parse(svgContent);

        // Write the background SVG
        File backgroundFile = new File(drawableDirectory, resourceName + "_background.xml");
        deriveAdaptiveAndroidFromSvgElement(logger, document, options.getBackground(), backgroundFile, false);

        // Write the foreground SVG
        File foregroundFile = new File(drawableDirectory, resourceName + "_foreground.xml");
        deriveAdaptiveAndroidFromSvgElement(logger, document, options.getForeground(), foregroundFile, false);

        // Write the monochrome SVG
        File monochromeFile = new File(drawableDirectory, resourceName + "_monochrome.xml");
        deriveAdaptiveAndroidFromSvgElement(logger, document, options.getForeground(), monochromeFile, true);

        File adaptiveIcon = new File(drawableV26Directory, resourceName + ".xml");
        writeAdaptiveIcon(
                adaptiveIcon,
                "@drawable/" + resourceName + "_foreground",
                "@drawable/" + resourceName + "_background",
                "@drawable/" + resourceName + "_monochrome"
        );
    }

    private void resizeSvg(Document document, int width, int height) {
        Element svg = document.getDocumentElement();
        svg.setAttribute("width", Integer.toString(width));
        svg.setAttribute("height", Integer.toString(height));
    }

    private void recolorElement(Element el, String newColor) {
        el.setAttribute("fill", newColor);
    }

    private void deriveAdaptiveAndroidFromSvgElement(
            Logger logger,
            Document svg,
            String id,
            File out,
            boolean monochrome
    ) throws Exception {
        // Clone the SVG and filter it to only contain the element with the given id
        Document copy = (Document) svg.cloneNode(true);
        resizeSvg(copy, 108, 108);
        filterSvg(copy, id);

        if (monochrome) {
            recolorElement((Element) findNodeById(copy, id), "#FFFFFF");
        }

        // Convert the SVG to an Android vector drawable
        convertSvgToAndroid(logger, copy, out);
    }

    private void filterSvg(Document toFilter, String keep) throws XPathExpressionException {
        Node nodeToKeep = findNodeById(toFilter, keep);
        Node root = toFilter.getDocumentElement();

        // Remove all top level g elements
        NodeList children = root.getChildNodes();
        for (int i = 0; i < children.getLength(); i++) {
            Node child = children.item(i);
            if ("g".equals(child.getNodeName())) {
                root.removeChild(child);
            }
        }

        // Add a g element with the el to keep
        Node g = toFilter.createElement("g");
        g.appendChild(nodeToKeep);

        root.appendChild(g);
    }

    private Node findNodeById(Node start, String id) throws XPathExpressionException {
        return xPathFactory.newXPath().evaluateExpression("//*[@id='" + id + "']", start, Node.class);
    }

    private void convertSvgToAndroid(Logger logger, Document svg, File outFile) throws Exception {
        // Write the SVG to a temporary file
        Path tempFile = Files.createTempFile("dragonclaw-android-icon", ".svg");
        Files.writeString(tempFile, XmlUtils.toXml(svg));

        try {
            convertSvgToAndroid(logger, tempFile, outFile);
        } finally {
            Files.delete(tempFile);
        }
    }

    private void convertSvgToAndroid(Logger logger, Path input, File outFile) throws Exception {
        try (OutputStream out = new FileOutputStream(outFile)) {
            // Convert the SVG to XML
            String errors = Svg2Vector.parseSvgToXml(input, out);

            if (errors != null) {
                logger.warn("Failed to cleanly convert SVG to XML: {}", errors);
            }

            out.flush();

            if (outFile.length() == 0) {
                throw new IOException("Failed to convert SVG to XML");
            }
        } catch (Exception e) {
            if (!outFile.delete()) {
                logger.warn("Failed to delete partially written file {}", outFile.getAbsolutePath());
            }

            throw e;
        }
    }

    private void writeAdaptiveIcon(
            File out,
            String foreground,
            String background,
            String monochrome
    ) throws IOException {
        Document document = XmlUtils.createDocument(true);

        // Root element
        Element adaptiveIcon = document.createElement("adaptive-icon");
        adaptiveIcon.setAttribute("xmlns:android", "http://schemas.android.com/apk/res/android");
        document.appendChild(adaptiveIcon);

        // Background
        Element backgroundElement = document.createElement("background");
        backgroundElement.setAttribute("android:drawable", background);
        adaptiveIcon.appendChild(backgroundElement);

        // Foreground with insets
        Element foregroundElement = document.createElement("foreground");
        Element foregroundInsets = foregroundInsets(document);
        foregroundInsets.setAttribute("android:drawable", foreground);

        foregroundElement.appendChild(foregroundInsets);
        adaptiveIcon.appendChild(foregroundElement);

        // Monochrome with insets
        Element monochromeElement = document.createElement("monochrome");
        Element monochromeInsets = foregroundInsets(document);
        monochromeInsets.setAttribute("android:drawable", monochrome);

        monochromeElement.appendChild(monochromeInsets);
        adaptiveIcon.appendChild(monochromeElement);

        Files.writeString(out.toPath(), XmlUtils.toXml(document));
    }

    private Element foregroundInsets(Document document) {
        Element foregroundInsets = document.createElement("inset");
        foregroundInsets.setAttribute("android:insetLeft", "21dp");
        foregroundInsets.setAttribute("android:insetTop", "21dp");
        foregroundInsets.setAttribute("android:insetRight", "21dp");
        foregroundInsets.setAttribute("android:insetBottom", "21dp");

        return foregroundInsets;
    }
}
