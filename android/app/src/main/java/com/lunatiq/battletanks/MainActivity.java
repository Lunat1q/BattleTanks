package com.lunatiq.battletanks;

import android.content.pm.ApplicationInfo;
import android.os.Bundle;
import android.system.ErrnoException;
import android.system.Os;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.WindowManager;

import com.google.androidgamesdk.GameActivity;

/** GameActivity loads libbattle_tanks.so (manifest meta-data) and runs the Rust game. */
public class MainActivity extends GameActivity {
    private SteamPads steam;
    private PadHub pads;

    /** Debug-build smoke hook: `adb shell am start -e BT_SHOT shot.png -e BT_STAGE 2 ...` (see CLAUDE.md). */
    private static final String[] SMOKE_ENV = {"BT_SHOT", "BT_STAGE", "BT_SHOT_AT", "BT_SCREEN", "BT_CITY", "BT_VIEW", "BT_MUTE"};

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        boolean debuggable = (getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0;
        if (debuggable && getIntent() != null) {
            for (String key : SMOKE_ENV) {
                String value = getIntent().getStringExtra(key);
                if (value == null) continue;
                // The screenshot lands in the app's external files dir, where `adb pull` can reach it.
                if (key.equals("BT_SHOT")) value = getExternalFilesDir(null) + "/" + value;
                try {
                    Os.setenv(key, value, true);
                } catch (ErrnoException e) {
                    throw new RuntimeException(e);
                }
            }
        }
        super.onCreate(savedInstanceState); // starts the native game thread, which reads the env
        // No screensaver mid-game: the TV's dream would hide (and so freeze) the game, e.g. during the title attract or a long pause.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        steam = new SteamPads(this);
        pads = new PadHub(this, steam);
    }

    @Override
    protected void onResume() {
        super.onResume();
        pads.resume();
        steam.start();
    }

    @Override
    protected void onPause() {
        // Releasing the Steam Controller disconnects it in-game, which auto-pauses a running stage.
        steam.stop();
        pads.pause();
        super.onPause();
    }

    // Pads and the TV remote are taken here, before GameActivity forwards input to winit (which drops joystick axes).
    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        return pads.onKey(event) || super.dispatchKeyEvent(event);
    }

    @Override
    public boolean dispatchGenericMotionEvent(MotionEvent event) {
        return pads.onMotion(event) || super.dispatchGenericMotionEvent(event);
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode == SteamPads.BLUETOOTH_REQUEST) {
            steam.startBluetooth();
        }
    }
}
