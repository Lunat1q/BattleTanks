package com.lunatiq.battletanks;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

/**
 * One state report from the 2026 Steam Controller ("Triton"), whatever the transport. Pure (no Android APIs) so
 * it is unit-tested on the desktop JVM. Protocol from SDL's SDL_hidapi_steam_triton.c / controller_structs.h
 * (zlib); first written by this project's owner for moonlight-android PR #623.
 */
final class TritonReport {
    // Trackpad bits in the button word (measured on hardware; the click fires past ~10% pressure)
    static final int LPAD_TOUCH = 0x02000000, LPAD_CLICK = 0x04000000;
    static final int RPAD_TOUCH = 0x00200000, RPAD_CLICK = 0x00400000;

    /** Button-word masks → Android KeyEvent keycodes, which Rust maps like any other pad (android_pads.rs). */
    static final int[][] BUTTONS = {
            {0x00000001, 96},                // A → BUTTON_A
            {0x00000002, 97},                // B
            {0x00000004, 99},                // X
            {0x00000008, 100},               // Y
            {0x00000020 | RPAD_CLICK, 107},  // right stick click, or a right pad click → THUMBR
            {0x00000040, 108},               // menu (SDL: Start) → BUTTON_START
            {0x00000200, 103},               // RB → R1
            {0x00000400, 20},                // D-pad down
            {0x00000800, 22},                // D-pad right
            {0x00001000, 21},                // D-pad left
            {0x00002000, 19},                // D-pad up
            {0x00004000, 109},               // view (SDL: Back) → BUTTON_SELECT
            {0x00008000 | LPAD_CLICK, 106},  // left stick click, or a left pad click → THUMBL
            {0x00010000, 110},               // Steam → BUTTON_MODE
            {0x00080000, 102},               // LB → L1
            // ponytail: back paddles (R4 0x80, R5 0x100, L4 0x20000, L5 0x40000) unmapped; the game has no use yet
    };

    int buttons;
    /** Android conventions: sticks -1..1 with Y down-positive, triggers 0..1. */
    float lx, ly, rx, ry, lt, rt;

    /** Parses a TritonMTUNoQuat_t (0x42/0x45) or TritonMTUNoQuat32TS_t (0x47) body at `off`; null if too short. */
    static TritonReport parse(byte[] data, int off, int len) {
        if (len < 17) {
            return null;
        }
        ByteBuffer b = ByteBuffer.wrap(data, off, len).slice().order(ByteOrder.LITTLE_ENDIAN);
        TritonReport r = new TritonReport();
        r.buttons = b.getInt(1);
        r.lt = Math.max(0, b.getShort(5)) / 32767.0f;
        r.rt = Math.max(0, b.getShort(7)) / 32767.0f;
        // The controller reports Y up-positive
        r.lx = b.getShort(9) / 32767.0f;
        r.ly = -b.getShort(11) / 32767.0f;
        r.rx = b.getShort(13) / 32767.0f;
        r.ry = -b.getShort(15) / 32767.0f;
        if (len >= 29) {
            if ((r.buttons & LPAD_TOUCH) != 0) {
                r.trackpad(true, b.getShort(17), b.getShort(19));
            }
            if ((r.buttons & RPAD_TOUCH) != 0) {
                r.trackpad(false, b.getShort(23), b.getShort(25));
            }
        }
        return r;
    }

    /** A touched pad drives the stick on its side by absolute finger position; whichever is pushed further wins. */
    private void trackpad(boolean left, short rawX, short rawY) {
        // Pads are square: pull the corners onto the stick's circle
        float x = rawX / 32767.0f, y = -rawY / 32767.0f;
        float m = (float) Math.hypot(x, y);
        if (m > 1) {
            x /= m;
            y /= m;
            m = 1;
        }
        if (left && m > Math.hypot(lx, ly)) {
            lx = x;
            ly = y;
        } else if (!left && m > Math.hypot(rx, ry)) {
            rx = x;
            ry = y;
        }
    }

    /** Everything released (controller dropped off), so nothing sticks held. */
    static final TritonReport NEUTRAL = new TritonReport();
}
