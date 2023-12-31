function Run-BuildCommand {
    param (
        [Parameter(Mandatory)]
        [string]$Command,

        [Parameter(Mandatory)]
        [string[]]$Arguments
    )

    $process = Start-Process -NoNewWindow -PassThru -Wait -FilePath $Command -ArgumentList $Arguments

    if ($process.ExitCode -ne 0) {
        throw "Command '$Command' failed with exit code $($process.ExitCode)"
    }
}

function Convert-ToRtf {
    param (
        [Parameter(Mandatory)]
        [string]$InputFile,

        [Parameter(Mandatory)]
        [string]$OutputFile
    )

    $TextBox = New-Object -TypeName System.Windows.Forms.RichTextBox
    $TextBox.LoadFile($InputFile, [System.Windows.Forms.RichTextBoxStreamType]::PlainText)
    $TextBox.SaveFile($OutputFile)

    Remove-Variable -Name TextBox
}

[void] [System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms")

$ErrorActionPreference = "Stop"

Run-BuildCommand -Command cargo -Arguments "build", "--release", "--target", "x86_64-pc-windows-msvc"

Convert-ToRtf -InputFile .\LICENSE -OutputFile .\target\x86_64-pc-windows-msvc\release\LICENSE.rtf

Run-BuildCommand -Command wix -Arguments "extension", "add", "WixToolset.UI.wixext"
Run-BuildCommand -Command wix -Arguments "extension", "add", "WixToolset.Firewall.wixext"
Run-BuildCommand -Command wix -Arguments @(
    "build",
    "-arch", "x64",
    ".\installer.wxs",
    "-ext", "WixToolset.UI.wixext", "-ext",
    "WixToolset.Firewall.wixext",
    "-out", "target/x86_64-pc-windows-msvc/release/dragon-claw-agent-installer.msi"
)
