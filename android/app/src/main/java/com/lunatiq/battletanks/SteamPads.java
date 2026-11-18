package com.lunatiq.battletanks;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.app.PendingIntent;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.hardware.usb.UsbDevice;
import android.hardware.usb.UsbDeviceConnection;
import android.hardware.usb.UsbInterface;
import android.hardware.usb.UsbManager;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.util.SparseArray;

import androidx.core.content.ContextCompat;

/**
 * Finds 2026 Steam Controllers (bonded over Bluetooth LE, or on the USB puck/cable) while the game is in front,
 * and lets them go when it isn't, so the controller is a normal mouse/keyboard again everywhere else.
 */
final class SteamPads {
    private static final String TAG = "BattleTanks";
    private static final String USB_PERMISSION = "com.lunatiq.battletanks.USB_PERMISSION";
    static final int BLUETOOTH_REQUEST = 1;
    /** Our pad ids live above Android's InputDevice ids. */
    private static final int FIRST_ID = 0x40000000;

    private final Activity activity;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final SparseArray<SteamPad> pads = new SparseArray<>();
    private int nextId = FIRST_ID;
    private boolean running, askedBluetooth;

    SteamPads(Activity activity) {
        this.activity = activity;
    }

    /** True while one of our drivers owns a controller: its lizard-mode key/mouse events are then swallowed. */
    synchronized boolean active() {
        return pads.size() > 0;
    }

    void start() {
        running = true;
        IntentFilter filter = new IntentFilter();
        filter.addAction(BluetoothDevice.ACTION_ACL_CONNECTED);
        filter.addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED);
        filter.addAction(USB_PERMISSION);
        ContextCompat.registerReceiver(activity, receiver, filter, ContextCompat.RECEIVER_NOT_EXPORTED);
        startBluetooth();
        UsbManager usb = (UsbManager) activity.getSystemService(Context.USB_SERVICE);
        if (usb != null) {
            for (UsbDevice dev : usb.getDeviceList().values()) {
                claimUsb(dev);
            }
        }
    }

    void stop() {
        running = false;
        activity.unregisterReceiver(receiver);
        SteamPad[] all;
        synchronized (this) {
            all = new SteamPad[pads.size()];
            for (int i = 0; i < all.length; i++) {
                all[i] = pads.valueAt(i);
            }
        }
        for (SteamPad p : all) {
            p.stop();
        }
    }

    synchronized void removed(SteamPad pad) {
        pads.remove(pad.id);
    }

    /** Rumble for `ms` milliseconds; strength 0..255 per motor. */
    void rumble(int id, int strong, int weak, int ms) {
        SteamPad pad;
        synchronized (this) {
            pad = pads.get(id);
        }
        if (pad == null) {
            return;
        }
        pad.rumble((short) (strong * 257), (short) (weak * 257));
        handler.postDelayed(() -> pad.rumble((short) 0, (short) 0), ms);
    }

    boolean hasBluetoothPermission() {
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.S
                || activity.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED;
    }

    /** Connects every bonded Steam Controller; asks for the Bluetooth permission once per launch if needed. */
    @SuppressLint("MissingPermission")
    void startBluetooth() {
        if (!running) {
            return;
        }
        if (!hasBluetoothPermission()) {
            if (!askedBluetooth) {
                askedBluetooth = true;
                activity.requestPermissions(new String[] {Manifest.permission.BLUETOOTH_CONNECT}, BLUETOOTH_REQUEST);
            }
            return;
        }
        BluetoothManager bt = (BluetoothManager) activity.getSystemService(Context.BLUETOOTH_SERVICE);
        BluetoothAdapter adapter = bt != null ? bt.getAdapter() : null;
        if (adapter == null || !adapter.isEnabled()) {
            return;
        }
        for (BluetoothDevice dev : adapter.getBondedDevices()) {
            if (SteamPadBle.isSteamController(dev)) {
                connectBle(dev);
            }
        }
    }

    private void connectBle(BluetoothDevice device) {
        synchronized (this) {
            for (int i = 0; i < pads.size(); i++) {
                SteamPad p = pads.valueAt(i);
                if (p instanceof SteamPadBle && ((SteamPadBle) p).getAddress().equals(device.getAddress())) {
                    return;
                }
            }
        }
        add(new SteamPadBle(activity, device, nextId++, this));
    }

    private void claimUsb(UsbDevice dev) {
        if (!running || !SteamPadUsb.canClaimDevice(dev)) {
            return;
        }
        UsbManager usb = (UsbManager) activity.getSystemService(Context.USB_SERVICE);
        if (!usb.hasPermission(dev)) {
            Intent intent = new Intent(USB_PERMISSION).setPackage(activity.getPackageName());
            usb.requestPermission(dev, PendingIntent.getBroadcast(activity, 0, intent, PendingIntent.FLAG_MUTABLE));
            return;
        }
        UsbDeviceConnection conn = usb.openDevice(dev);
        if (conn == null) {
            Log.w(TAG, "Steam Controller USB: openDevice failed");
            return;
        }
        for (UsbInterface iface : SteamPadUsb.controllerInterfaces(dev)) {
            add(new SteamPadUsb(dev, conn, iface, nextId++, this));
        }
    }

    private void add(SteamPad pad) {
        synchronized (this) {
            pads.put(pad.id, pad);
        }
        if (!pad.start()) {
            removed(pad);
        }
    }

    private final BroadcastReceiver receiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            String action = intent.getAction();
            if (BluetoothDevice.ACTION_ACL_CONNECTED.equals(action)) {
                // A bonded controller came back (e.g. powered on mid-game)
                BluetoothDevice dev = intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE);
                if (dev != null && hasBluetoothPermission() && SteamPadBle.isSteamController(dev)) {
                    connectBle(dev);
                }
            } else if (UsbManager.ACTION_USB_DEVICE_ATTACHED.equals(action) || USB_PERMISSION.equals(action)) {
                UsbDevice dev = intent.getParcelableExtra(UsbManager.EXTRA_DEVICE);
                if (dev != null && !intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, true)) {
                    return; // the player said no
                }
                if (dev != null) {
                    claimUsb(dev);
                }
            }
        }
    };
}
