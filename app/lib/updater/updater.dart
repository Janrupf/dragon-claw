import 'dart:convert';
import 'dart:convert' as convert;
import 'dart:io';

import 'package:logging/logging.dart';
import 'package:package_info_plus/package_info_plus.dart';

final _logger = Logger("updater");

/// Updater for the app
class AppUpdater {
  /// URL to fetch the latest metadata from
  static const String _url =
      "https://static-content.janrupf.net/updates/dragon-claw/android-app.json";

  final HttpClient _client;
  Future<AvailableUpdate?>? _runningUpdateCheck;

  AppUpdater() : _client = HttpClient()..userAgent = "DragonClawAndroid";

  /// Check for updates
  Future<AvailableUpdate?> checkForUpdates() {
    _runningUpdateCheck ??= _doCheckForUpdates().whenComplete(() {
      _runningUpdateCheck = null;
    });

    return _runningUpdateCheck!;
  }

  Future<AvailableUpdate?> _doCheckForUpdates() async {
    final packageInfo = await PackageInfo.fromPlatform();

    _logger.info("Current version is ${packageInfo.version}");
    _logger.info("Fetching update...");

    // Request the URL and read the response as a string
    final request = await _client.getUrl(Uri.parse(_url));
    request.followRedirects = true;

    final response = await request.close();

    if (response.statusCode != 200) {
      throw UpdateException("Failed to fetch update: ${response.statusCode}");
    }

    final body = await response.transform(utf8.decoder).join();

    // Parse the JSON
    final json = convert.jsonDecode(body);
    if (json is! Map) {
      throw FormatException("Expected JSON object", body);
    }

    // Extract the required data
    final latestVersion = json["latestVersion"];
    if (latestVersion is! String) {
      throw FormatException("Expected latestVersion to be a string", body);
    }

    final downloadUrl = json["downloadUrl"];
    if (downloadUrl is! String) {
      throw FormatException("Expected downloadUrl to be a string", body);
    }

    _logger.info("Latest version is $latestVersion");

    if (latestVersion == packageInfo.version) {
      // Up to date
      return null;
    }

    return AvailableUpdate(latestVersion, downloadUrl);
  }
}

class UpdateException extends IOException {
  final String message;

  UpdateException(this.message);

  @override
  String toString() => message;
}

/// An available update.
class AvailableUpdate {
  final String version;
  final String url;

  const AvailableUpdate(this.version, this.url);
}
