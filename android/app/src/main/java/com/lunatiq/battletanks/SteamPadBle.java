package com.lunatiq.battletanks;

import android.annotation.SuppressLint;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothGatt;
import android.bluetooth.BluetoothGattCallback;
import android.bluetooth.BluetoothGattCharacteristic;
import android.bluetooth.BluetoothGattDescriptor;
import android.bluetooth.BluetoothGattService;
import android.bluetooth.BluetoothProfile;
import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.util.ArrayDeque;
import java.util.Arrays;
import java.util.UUID;

/**
 * Steam Controller (2026) over Bluetooth LE, talking to Valve's GATT service directly. Transport from SDL's
 * HIDDeviceBLESteamController.java (zlib), via the owner's moonlight-android PR #623 (SteamControllerBle).
 */
@SuppressLint("MissingPermission") // SteamPads only creates us once BLUETOOTH_CONNECT is granted
final class SteamPadBle extends SteamPad {
    private static final String TAG = "BattleTanks";
    private static final int TRITON_BLE_PID = 0x1303;

    private static final UUID SERVICE = UUID.fromString("100F6C32-1735-4313-B402-38567131E5F3");
    private static final UUID INPUT_0x45 = UUID.fromString("100F6C7A-1735-4313-B402-38567131E5F3");
    private static final UUID INPUT_0x47 = UUID.fromString("100F6C7C-1735-4313-B402-38567131E5F3");
    private static final UUID FEATURE_REPORT = UUID.fromString("100F6C34-1735-4313-B402-38567131E5F3");
    // Output report N lives at characteristic 100F6C(N + 0x35); rumble is report 0x80
    private static final UUID RUMBLE_REPORT = UUID.fromString("100F6CB5-1735-4313-B402-38567131E5F3");
    private static final UUID CCCD = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb");

    private static final int GATT_OP_TIMEOUT_MS = 1000;

    private final Context context;
    private final BluetoothDevice device;
    private final Handler handler = new Handler(Looper.getMainLooper());

    private interface GattOp { boolean start(); }
    private final ArrayDeque<GattOp> ops = new ArrayDeque<>();
    private boolean opInFlight, rumbleQueued;

    private BluetoothGatt gatt;
    private BluetoothGattCharacteristic featureChr, rumbleChr;
    private volatile boolean ready, stopped;

    static boolean isSteamController(BluetoothDevice dev) {
        // Same match SDL uses: the 2026 controller advertises as "Steam Ctrl ..."
        String name = dev.getName();
        return name != null && name.startsWith("Steam Ctrl") && (dev.getType() & BluetoothDevice.DEVICE_TYPE_LE) != 0;
    }

    SteamPadBle(Context context, BluetoothDevice device, int id, SteamPads owner) {
        super(id, TRITON_BLE_PID, owner);
        this.context = context;
        this.device = device;
    }

    String getAddress() {
        return device.getAddress();
    }

    @Override
    boolean start() {
        Log.i(TAG, "Steam Controller BLE: connecting to " + device.getAddress());
        gatt = device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE);
        return gatt != null;
    }

    // Main thread only (SteamPads calls it there, and we post our own calls)
    @Override
    void stop() {
        synchronized (this) {
            if (stopped) {
                return;
            }
            stopped = true;
            ops.clear();
        }
        handler.removeCallbacksAndMessages(null);
        if (gatt != null) {
            gatt.disconnect();
            gatt.close();
        }
        notifyRemoved();
    }

    @Override
    void rumble(short lowFreqMotor, short highFreqMotor) {
        this.lowFreqMotor = lowFreqMotor;
        this.highFreqMotor = highFreqMotor;
        handler.removeCallbacks(rumbleRefresh);
        handler.post(rumbleRefresh);
    }

    private final Runnable rumbleRefresh = new Runnable() {
        @Override
        public void run() {
            queueRumble();
            if (lowFreqMotor != 0 || highFreqMotor != 0) {
                handler.postDelayed(this, RUMBLE_RESEND_MS);
            }
        }
    };

    private final Runnable maintainSettings = new Runnable() {
        @Override
        public void run() {
            enqueue(() -> {
                featureChr.setValue(SETTINGS);
                return gatt.writeCharacteristic(featureChr);
            });
            handler.postDelayed(this, SETTINGS_INTERVAL_MS);
        }
    };

    private void onReady() {
        if (ready || stopped) {
            return;
        }
        ready = true;
        Log.i(TAG, "Steam Controller BLE: ready");
        handler.post(maintainSettings);
        notifyAdded();
    }

    private void queueRumble() {
        synchronized (this) {
            if (rumbleQueued || rumbleChr == null) {
                return;
            }
            rumbleQueued = true;
        }
        enqueue(() -> {
            synchronized (this) {
                rumbleQueued = false;
            }
            // SDL's BLE path drops the trailing right gain byte, so match it
            rumbleChr.setValue(Arrays.copyOf(rumbleBody(), 8));
            return gatt.writeCharacteristic(rumbleChr);
        });
    }

    // Android allows one outstanding GATT operation at a time, so everything goes through here.
    private synchronized void enqueue(GattOp op) {
        ops.add(op);
        pump();
    }

    private synchronized void opDone() {
        handler.removeCallbacks(opTimeout);
        opInFlight = false;
        pump();
    }

    // ponytail: some stacks (Fire TV) silently drop callbacks; time out so the queue never wedges.
    // A late callback after a timeout can complete the next op early, which is harmless here.
    private final Runnable opTimeout = this::opDone;

    private synchronized void pump() {
        while (!opInFlight && !stopped && !ops.isEmpty()) {
            opInFlight = ops.poll().start();
            if (opInFlight) {
                handler.postDelayed(opTimeout, GATT_OP_TIMEOUT_MS);
            }
        }
    }

    private final BluetoothGattCallback callback = new BluetoothGattCallback() {
        @Override
        public void onConnectionStateChange(BluetoothGatt g, int status, int newState) {
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                handler.post(() -> {
                    if (!stopped) {
                        g.discoverServices();
                    }
                });
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                Log.i(TAG, "Steam Controller BLE: disconnected (status " + status + ")");
                handler.post(SteamPadBle.this::stop);
            }
        }

        @Override
        public void onServicesDiscovered(BluetoothGatt g, int status) {
            BluetoothGattService svc = g.getService(SERVICE);
            BluetoothGattCharacteristic input = null;
            if (svc != null) {
                input = svc.getCharacteristic(INPUT_0x45);
                if (input == null) {
                    input = svc.getCharacteristic(INPUT_0x47);
                }
                featureChr = svc.getCharacteristic(FEATURE_REPORT);
                rumbleChr = svc.getCharacteristic(RUMBLE_REPORT);
            }
            if (status != BluetoothGatt.GATT_SUCCESS || input == null || featureChr == null) {
                Log.w(TAG, "Steam Controller BLE: Valve service not found (status " + status + ")");
                handler.post(SteamPadBle.this::stop);
                return;
            }

            // The controller needs Data Length Extensions; 517 is the value that makes Android enable them
            enqueue(() -> g.requestMtu(517));

            final BluetoothGattCharacteristic inputChr = input;
            enqueue(() -> {
                BluetoothGattDescriptor cccd = inputChr.getDescriptor(CCCD);
                if (cccd == null || !g.setCharacteristicNotification(inputChr, true)) {
                    return false;
                }
                cccd.setValue(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE);
                return g.writeDescriptor(cccd);
            });
        }

        @Override
        public void onMtuChanged(BluetoothGatt g, int mtu, int status) {
            opDone();
        }

        @Override
        public void onDescriptorWrite(BluetoothGatt g, BluetoothGattDescriptor descriptor, int status) {
            opDone();
            handler.post(SteamPadBle.this::onReady);
        }

        @Override
        public void onCharacteristicWrite(BluetoothGatt g, BluetoothGattCharacteristic characteristic, int status) {
            opDone();
        }

        @Override
        public void onCharacteristicChanged(BluetoothGatt g, BluetoothGattCharacteristic characteristic) {
            if (!ready) {
                // Some stacks never deliver onDescriptorWrite even though notifications are flowing
                handler.post(SteamPadBle.this::onReady);
                return;
            }
            // GATT notifications carry the report body without its ID byte
            byte[] data = characteristic.getValue();
            if (data != null) {
                handleState(data, 0, data.length);
            }
        }
    };
}
