package com.lunatiq.battletanks;

/**
 * One Steam Controller (2026) driven by our own driver, because Android only sees its mouse/keyboard
 * ("lizard") mode. Subclasses are the transports (BLE, USB puck/cable); this base turns state reports into the
 * same raw keycode/axis events Android pads produce, so Rust treats it like any other gamepad.
 * Ported from the owner's moonlight-android PR #623 (SteamTritonController).
 */
abstract class SteamPad {
    static final int VALVE_VID = 0x28de;

    // Input report IDs
    static final int ID_STATE = 0x42;
    static final int ID_STATE_BLE = 0x45;
    static final int ID_STATE_TIMESTAMP = 0x47;
    static final int ID_WIRELESS_STATUS_X = 0x46;
    static final int ID_WIRELESS_STATUS = 0x79;
    static final int WIRELESS_DISCONNECT = 1;
    static final int ID_OUT_RUMBLE = 0x80;

    // Feature report body (after the report ID byte): ID_SET_SETTINGS_VALUES with SETTING_LIZARD_MODE(9)=OFF and
    // SETTING_IMU_MODE(48)=RAW_ACCEL|RAW_GYRO. The controller turns lizard mode back on by itself if this isn't
    // resent within a few seconds, so it is a normal mouse/keyboard again once the game is closed.
    static final byte[] SETTINGS = new byte[62];
    static {
        byte[] s = {(byte) 0x87, 6, 9, 0, 0, 48, 0x18, 0};
        System.arraycopy(s, 0, SETTINGS, 0, s.length);
    }
    static final int SETTINGS_INTERVAL_MS = 3000;
    static final int RUMBLE_RESEND_MS = 40; // hardware rumble safety timeout is ~50 ms

    // Android axis ids for lx, ly, rx, ry, lt, rt (MotionEvent.AXIS_X, Y, Z, RZ, LTRIGGER, RTRIGGER)
    private static final int[] AXES = {0, 1, 11, 14, 17, 18};

    final int id;
    private final int productId;
    private final SteamPads owner;
    private int lastButtons;
    private final float[] lastAxes = new float[6];
    private boolean added;
    protected volatile short lowFreqMotor, highFreqMotor;

    SteamPad(int id, int productId, SteamPads owner) {
        this.id = id;
        this.productId = productId;
        this.owner = owner;
    }

    abstract boolean start();

    abstract void stop();

    /** Motor speeds 0..65535; held until set back to 0 (the transports keep resending). */
    abstract void rumble(short lowFreqMotor, short highFreqMotor);

    // MsgHapticRumble without the report ID: type, intensity(2), left speed(2), left gain, right speed(2), right gain
    protected byte[] rumbleBody() {
        int low = lowFreqMotor & 0xFFFF, high = highFreqMotor & 0xFFFF;
        return new byte[] {0, 0, 0, (byte) low, (byte) (low >> 8), 0, (byte) high, (byte) (high >> 8), 0};
    }

    protected void handleState(byte[] data, int off, int len) {
        TritonReport r = TritonReport.parse(data, off, len);
        if (r != null) {
            report(r);
        }
    }

    protected void reportNeutral() {
        report(TritonReport.NEUTRAL);
    }

    private synchronized void report(TritonReport r) {
        for (int[] b : TritonReport.BUTTONS) {
            boolean down = (r.buttons & b[0]) != 0;
            if (down != ((lastButtons & b[0]) != 0)) {
                PadHub.send(id, PadHub.KEY, b[1], down ? 1 : 0);
            }
        }
        lastButtons = r.buttons;
        float[] axes = {r.lx, r.ly, r.rx, r.ry, r.lt, r.rt};
        for (int i = 0; i < axes.length; i++) {
            if (axes[i] != lastAxes[i]) {
                lastAxes[i] = axes[i];
                PadHub.send(id, PadHub.AXIS, AXES[i], axes[i]);
            }
        }
    }

    protected synchronized void notifyAdded() {
        if (!added) {
            added = true;
            PadHub.connect(id, VALVE_VID, productId, false);
        }
    }

    protected synchronized void notifyRemoved() {
        if (added) {
            added = false;
            PadHub.send(id, PadHub.DISCONNECT, 0, 0);
        }
        owner.removed(this);
    }
}
