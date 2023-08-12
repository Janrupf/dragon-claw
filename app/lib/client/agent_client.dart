import 'dart:io';

import 'package:dragon_claw/generated/google/protobuf/empty.pb.dart' as rpc;
import 'package:dragon_claw/generated/service.pbgrpc.dart' as rpc;
import 'package:flutter/material.dart';
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

  /// Query the agent for the supported power actions.
  Future<Set<PowerAction>> getSupportedPowerActions() async {
    var response = await _client.getSupportedPowerActions(rpc.Empty());
    return response.actions.map(PowerAction.fromRpc).toSet();
  }

  /// Perform a power action on the system.
  Future<void> performPowerAction(PowerAction action) async {
    await _client
        .performPowerAction(rpc.PowerActionRequest()..action = action.rpcValue);
  }
}

/// Power actions that can be performed on the system.
enum PowerAction {
  /// Power off the system.
  powerOff,

  /// Reboot the system.
  reboot,

  /// Reboot the system into the firmware.
  rebootToFirmware,

  /// Lock the screen.
  lock,

  /// Log out the current user.
  logOut,

  /// Suspend the system.
  suspend,

  /// Hibernate the system.
  hibernate,

  /// Hybrid suspend the system.
  hybridSuspend;

  /// Retrieves a human readable name for the power action.
  String get name {
    switch (this) {
      case PowerAction.powerOff:
        return "Power Off";
      case PowerAction.reboot:
        return "Reboot";
      case PowerAction.rebootToFirmware:
        return "Reboot to Firmware";
      case PowerAction.lock:
        return "Lock";
      case PowerAction.logOut:
        return "Log Out";
      case PowerAction.suspend:
        return "Suspend";
      case PowerAction.hibernate:
        return "Hibernate";
      case PowerAction.hybridSuspend:
        return "Hybrid Suspend";
    }
  }

  /// Retrieves a human readable description for the power action.
  String get description {
    switch (this) {
      case PowerAction.powerOff:
        return "Power off the system";
      case PowerAction.reboot:
        return "Reboot the system";
      case PowerAction.rebootToFirmware:
        return "Reboot the system into the firmware";
      case PowerAction.lock:
        return "Lock the screen";
      case PowerAction.logOut:
        return "Log out the current user";
      case PowerAction.suspend:
        return "Suspend the system";
      case PowerAction.hibernate:
        return "Hibernate the system";
      case PowerAction.hybridSuspend:
        return "Hybrid suspend the system";
    }
  }

  /// Retrieves an icon for the power action.
  IconData get icon {
    switch (this) {
      case PowerAction.powerOff:
        return Icons.power_settings_new;
      case PowerAction.reboot:
        return Icons.restart_alt;
      case PowerAction.rebootToFirmware:
        return Icons.settings;
      case PowerAction.lock:
        return Icons.lock;
      case PowerAction.logOut:
        return Icons.logout;
      case PowerAction.suspend:
        return Icons.bedtime;
      case PowerAction.hibernate:
        return Icons.bedtime;
      case PowerAction.hybridSuspend:
        return Icons.bedtime;
    }
  }

  /// Convert to an RPC value.
  rpc.PowerAction get rpcValue {
    switch (this) {
      case PowerAction.powerOff:
        return rpc.PowerAction.POWER_OFF;
      case PowerAction.reboot:
        return rpc.PowerAction.REBOOT;
      case PowerAction.rebootToFirmware:
        return rpc.PowerAction.REBOOT_TO_FIRMWARE;
      case PowerAction.lock:
        return rpc.PowerAction.LOCK;
      case PowerAction.logOut:
        return rpc.PowerAction.LOG_OUT;
      case PowerAction.suspend:
        return rpc.PowerAction.SUSPEND;
      case PowerAction.hibernate:
        return rpc.PowerAction.HIBERNATE;
      case PowerAction.hybridSuspend:
        return rpc.PowerAction.HYBRID_SUSPEND;
    }
  }

  /// Convert from an RPC value to a [PowerAction].
  static PowerAction fromRpc(rpc.PowerAction action) {
    switch (action) {
      case rpc.PowerAction.POWER_OFF:
        return PowerAction.powerOff;
      case rpc.PowerAction.REBOOT:
        return PowerAction.reboot;
      case rpc.PowerAction.REBOOT_TO_FIRMWARE:
        return PowerAction.rebootToFirmware;
      case rpc.PowerAction.LOCK:
        return PowerAction.lock;
      case rpc.PowerAction.LOG_OUT:
        return PowerAction.logOut;
      case rpc.PowerAction.SUSPEND:
        return PowerAction.suspend;
      case rpc.PowerAction.HIBERNATE:
        return PowerAction.hibernate;
      case rpc.PowerAction.HYBRID_SUSPEND:
        return PowerAction.hybridSuspend;
      default:
        throw ArgumentError("Unknown PowerAction: $action");
    }
  }
}
