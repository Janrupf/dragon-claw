import 'dart:io';

import 'package:dragon_claw/generated/google/protobuf/empty.pb.dart' as rpc;
import 'package:dragon_claw/generated/service.pbgrpc.dart' as rpc;
import 'package:grpc/grpc.dart';

/// Client to connect to the DragonClaw Agent
class DragonClawAgentClient {
  final InternetAddress address;
  final int port;

  final rpc.DragonClawAgentClient _client;

  /// Creates the client and connects the RPC
  DragonClawAgentClient(this.address, this.port)
      : _client = rpc.DragonClawAgentClient(
          ClientChannel(address,
              port: port,
              options: const ChannelOptions(
                credentials: ChannelCredentials.insecure(),
              )),
        );

  /// Sends a shutdown request to the system.
  Future<void> shutdownSystem() async {
    await _client.shutdownSystem(rpc.Empty());
  }
}
