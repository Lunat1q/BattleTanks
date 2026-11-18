package com.lunatiq.battletanks;

import android.content.Context;
import android.hardware.input.InputManager;
import android.os.Build;
import android.os.CombinedVibration;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.os.VibratorManager;
import android.util.SparseArray;
import android.view.Choreographer;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;

/**
 * Every controller and TV-remote event goes to Rust from here (src/android_pads.rs), because gilrs, Bevy's
 * gamepad backend, has no Android support (ADR 0006). Events carry raw Android keycodes/axis ids; Rust maps them.
 * Real keyboards are left alone and reach the game through winit as keys.
 */
final class PadHub implements InputManager.InputDeviceListener, Choreographer.FrameCallback {
    // Event kinds; must match android_pads.rs.
    static final int CONNECT = 0, DISCONNECT = 1, KEY = 2, AXIS = 3;
    private static final int REMOTE_FLAG = 1 << 31;

    static native void nativePad(int id, int kind, int code, float value);

    /** One pending rumble packed as id(32) | strong(8) | weak(8) | ms(16), or 0. */
    static native long nativePollRumble();

    private static final int[] AXES = {
            MotionEvent.AXIS_X, MotionEvent.AXIS_Y, MotionEvent.AXIS_Z, MotionEvent.AXIS_RZ,
            MotionEvent.AXIS_HAT_X, MotionEvent.AXIS_HAT_Y,
            MotionEvent.AXIS_LTRIGGER, MotionEvent.AXIS_RTRIGGER, MotionEvent.AXIS_BRAKE, MotionEvent.AXIS_GAS,
    };
    /** Keys a TV remote can drive the game with: D-pad, OK/Enter, Back, Menu, Play/Pause. */
    private static final int[] REMOTE_KEYS = {19, 20, 21, 22, 23, 66, 4, 82, 85};

    private final Context context;
    private final SteamPads steam;
    /** Android device id → last axis values; present once Rust has been told the device connected. */
    private final SparseArray<float[]> known = new SparseArray<>();

    PadHub(Context context, SteamPads steam) {
        this.context = context;
        this.steam = steam;
    }

    static void send(int id, int kind, int code, float value) {
        nativePad(id, kind, code, value);
    }

    static void connect(int id, int vendor, int product, boolean remote) {
        nativePad(id, CONNECT, ((vendor & 0x7FFF) << 16) | (product & 0xFFFF) | (remote ? REMOTE_FLAG : 0), 0);
    }

    void resume() {
        ((InputManager) context.getSystemService(Context.INPUT_SERVICE)).registerInputDeviceListener(this, null);
        Choreographer.getInstance().postFrameCallback(this);
    }

    void pause() {
        ((InputManager) context.getSystemService(Context.INPUT_SERVICE)).unregisterInputDeviceListener(this);
        Choreographer.getInstance().removeFrameCallback(this);
    }

    /**
     * A real controller has buttons and sticks. Remotes claim one or the other (the Sony Bravia's BT remote says
     * JOYSTICK, Android TV's virtual remotes say GAMEPAD), never both.
     */
    private static boolean isPad(InputDevice dev) {
        int s = dev.getSources();
        return (s & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD && (s & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK;
    }

    private static boolean isRemoteKey(int code) {
        for (int k : REMOTE_KEYS) {
            if (k == code) {
                return true;
            }
        }
        return false;
    }

    /** True if the event was ours (a pad or remote, or a Steam Controller we drive ourselves). */
    boolean onKey(KeyEvent e) {
        InputDevice dev = e.getDevice();
        if (dev == null) {
            return false;
        }
        if (dev.getVendorId() == SteamPad.VALVE_VID && steam.active()) {
            return true; // its lizard-mode duplicate; our driver reports the real state
        }
        int code = e.getKeyCode();
        boolean pad = isPad(dev) && (KeyEvent.isGamepadButton(code) || isRemoteKey(code));
        boolean remote = !pad && isRemoteKey(code) && dev.getKeyboardType() != InputDevice.KEYBOARD_TYPE_ALPHABETIC;
        if (!pad && !remote) {
            return false; // volume, home, real keyboards...
        }
        ensureConnected(dev, remote);
        if (e.getRepeatCount() == 0 && (e.getAction() == KeyEvent.ACTION_DOWN || e.getAction() == KeyEvent.ACTION_UP)) {
            nativePad(dev.getId(), KEY, code, e.getAction() == KeyEvent.ACTION_DOWN ? 1 : 0);
        }
        return true;
    }

    boolean onMotion(MotionEvent e) {
        InputDevice dev = e.getDevice();
        if (dev == null) {
            return false;
        }
        if (dev.getVendorId() == SteamPad.VALVE_VID && steam.active()) {
            return true;
        }
        if (!isPad(dev) || (e.getSource() & InputDevice.SOURCE_JOYSTICK) != InputDevice.SOURCE_JOYSTICK || e.getAction() != MotionEvent.ACTION_MOVE) {
            return false;
        }
        float[] last = ensureConnected(dev, false);
        for (int i = 0; i < AXES.length; i++) {
            float v = e.getAxisValue(AXES[i]);
            if (v != last[i]) {
                last[i] = v;
                nativePad(dev.getId(), AXIS, AXES[i], v);
            }
        }
        return true;
    }

    private float[] ensureConnected(InputDevice dev, boolean remote) {
        float[] last = known.get(dev.getId());
        if (last == null) {
            last = new float[AXES.length];
            known.put(dev.getId(), last);
            connect(dev.getId(), dev.getVendorId(), dev.getProductId(), remote);
        }
        return last;
    }

    @Override
    public void onInputDeviceAdded(int id) {}

    @Override
    public void onInputDeviceChanged(int id) {}

    @Override
    public void onInputDeviceRemoved(int id) {
        if (known.get(id) != null) {
            known.remove(id);
            nativePad(id, DISCONNECT, 0, 0);
        }
    }

    /** Each display frame: hand pending rumble to the pads. */
    @Override
    public void doFrame(long frameTimeNanos) {
        for (long r = nativePollRumble(); r != 0; r = nativePollRumble()) {
            int id = (int) (r >>> 32), strong = (int) (r >>> 24) & 0xFF, weak = (int) (r >>> 16) & 0xFF, ms = (int) r & 0xFFFF;
            if (id >= 0x40000000) {
                steam.rumble(id, strong, weak, ms);
            } else {
                vibrate(InputDevice.getDevice(id), strong, weak, ms);
            }
        }
        Choreographer.getInstance().postFrameCallback(this);
    }

    private static void vibrate(InputDevice dev, int strong, int weak, int ms) {
        if (dev == null || ms == 0) {
            return;
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            VibratorManager vm = dev.getVibratorManager();
            int[] ids = vm.getVibratorIds();
            if (ids.length == 0) {
                return;
            }
            // Xbox-style pads: first motor is the heavy (low-frequency) one
            CombinedVibration.ParallelCombination c = CombinedVibration.startParallel();
            int[] amps = {strong, weak};
            boolean any = false;
            for (int i = 0; i < ids.length && i < 2; i++) {
                if (amps[i] > 0) {
                    c.addVibrator(ids[i], VibrationEffect.createOneShot(ms, amps[i]));
                    any = true;
                }
            }
            if (any) {
                vm.vibrate(c.combine());
            }
        } else {
            Vibrator v = dev.getVibrator();
            if (v.hasVibrator()) {
                v.vibrate(VibrationEffect.createOneShot(ms, Math.max(1, Math.max(strong, weak))));
            }
        }
    }
}
