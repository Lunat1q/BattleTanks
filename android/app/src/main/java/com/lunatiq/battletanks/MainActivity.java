package com.lunatiq.battletanks;

import android.content.pm.ApplicationInfo;
import android.os.Bundle;
import android.system.ErrnoException;
import android.system.Os;
import android.view.WindowManager;

import com.google.androidgamesdk.GameActivity;

/** GameActivity loads libbattle_tanks.so (manifest meta-data) and runs the Rust game. */
public class MainActivity extends GameActivity {
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
    }
}
