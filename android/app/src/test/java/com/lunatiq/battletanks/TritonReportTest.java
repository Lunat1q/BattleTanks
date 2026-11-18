package com.lunatiq.battletanks;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNull;

import org.junit.Test;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public class TritonReportTest {
    private static ByteBuffer report(int size) {
        return ByteBuffer.allocate(size).order(ByteOrder.LITTLE_ENDIAN);
    }

    @Test
    public void parsesBleStateReport() {
        ByteBuffer b = report(45);
        b.putInt(1, 0x00000001 | 0x00010000 | 0x00002000); // A, Steam, D-up
        b.putShort(5, (short) 32767);  // left trigger full
        b.putShort(9, (short) 32767);  // left stick full right
        b.putShort(11, (short) 32767); // left stick full up
        TritonReport r = TritonReport.parse(b.array(), 0, 45);

        assertEquals(0x00000001 | 0x00010000 | 0x00002000, r.buttons);
        assertEquals(1.0f, r.lx, 1e-4);
        assertEquals(-1.0f, r.ly, 1e-4); // Android convention: Y down-positive
        assertEquals(1.0f, r.lt, 1e-4);
        assertEquals(0.0f, r.rt, 1e-4);
    }

    @Test
    public void parsesUsbReportAfterItsIdByte() {
        ByteBuffer b = report(46);
        b.put(0, (byte) SteamPad.ID_STATE);
        b.putInt(2, 0x00000002); // B
        b.putShort(10, (short) -32767); // left stick full left
        TritonReport r = TritonReport.parse(b.array(), 1, 45);

        assertEquals(0x00000002, r.buttons);
        assertEquals(-1.0f, r.lx, 1e-4);
    }

    @Test
    public void touchedLeftPadDrivesTheStickUnlessTheStickIsFurther() {
        ByteBuffer b = report(45);
        b.putInt(1, TritonReport.LPAD_TOUCH);
        b.putShort(17, (short) 32767); // finger at the pad's top-right corner
        b.putShort(19, (short) 32767);
        TritonReport r = TritonReport.parse(b.array(), 0, 45);
        float d = (float) Math.sqrt(0.5);
        assertEquals(d, r.lx, 1e-3); // corner pulled onto the circle
        assertEquals(-d, r.ly, 1e-3);

        b.putShort(9, (short) -32767); // stick pushed fully left: the stick wins
        r = TritonReport.parse(b.array(), 0, 45);
        assertEquals(-1.0f, r.lx, 1e-4);
    }

    @Test
    public void padClickCountsAsStickClick() {
        int[] thumbL = null;
        for (int[] m : TritonReport.BUTTONS) {
            if (m[1] == 106) thumbL = m;
        }
        assertEquals(TritonReport.LPAD_CLICK, thumbL[0] & TritonReport.LPAD_CLICK);
    }

    @Test
    public void shortReportIsIgnored() {
        assertNull(TritonReport.parse(new byte[10], 0, 10));
    }
}
