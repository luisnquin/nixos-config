import android.accessibilityservice.AccessibilityServiceInfo;
import android.app.UiAutomation;
import android.graphics.Rect;
import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import android.view.accessibility.AccessibilityWindowInfo;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.util.List;

public final class PhoneDump {
    public static void main(String[] args) throws Exception {
        Looper.prepareMainLooper();
        HandlerThread thread = new HandlerThread("phone-dump");
        thread.start();

        Class<?> connection = Class.forName("android.app.IUiAutomationConnection");
        Constructor<UiAutomation> create = UiAutomation.class.getDeclaredConstructor(Looper.class, connection);
        create.setAccessible(true);
        UiAutomation automation = create.newInstance(
                thread.getLooper(), Class.forName("android.app.UiAutomationConnection").getDeclaredConstructor().newInstance());

        hidden("connect", int.class).invoke(automation, UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);

        if (args.length > 0 && args[0].equals("pinch")) {
            pinch(automation, args);
            hidden("disconnect").invoke(automation);
            System.exit(0);
        }

        AccessibilityServiceInfo info = automation.getServiceInfo();
        info.flags |= AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS
                | AccessibilityServiceInfo.FLAG_INCLUDE_NOT_IMPORTANT_VIEWS;
        automation.setServiceInfo(info);

        StringBuilder out = new StringBuilder("<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><displays><display id=\"0\">");

        for (AccessibilityWindowInfo window : windows(automation)) {
            AccessibilityNodeInfo root = window.getRoot();
            if (root == null) {
                continue;
            }

            Rect frame = new Rect();
            window.getBoundsInScreen(frame);
            out.append("<window type=\"").append(type(window.getType())).append("\" bounds=\"").append(bounds(frame)).append("\">");
            out.append("<hierarchy rotation=\"0\">");
            node(root, frame, out);
            out.append("</hierarchy></window>");
        }

        out.append("</display></displays>");
        System.out.print(out);
        System.out.flush();

        hidden("disconnect").invoke(automation);
        System.exit(0);
    }

    private static void pinch(UiAutomation automation, String[] args) throws Exception {
        int display = Integer.parseInt(args[1]);
        long step = Long.parseLong(args[2]);
        int frames = (args.length - 3) / 4;
        if (frames < 2 || (args.length - 3) % 4 != 0) {
            throw new IllegalArgumentException("a pinch needs at least two steps of four coordinates");
        }

        float[][] at = new float[frames][4];
        for (int i = 0; i < frames; i++) {
            for (int j = 0; j < 4; j++) {
                at[i][j] = Float.parseFloat(args[3 + i * 4 + j]);
            }
        }

        int second = 1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT;
        long down = SystemClock.uptimeMillis();

        touch(automation, display, down, MotionEvent.ACTION_DOWN, 1, at[0]);
        touch(automation, display, down, MotionEvent.ACTION_POINTER_DOWN | second, 2, at[0]);

        for (int i = 1; i < frames; i++) {
            SystemClock.sleep(step);
            touch(automation, display, down, MotionEvent.ACTION_MOVE, 2, at[i]);
        }

        touch(automation, display, down, MotionEvent.ACTION_POINTER_UP | second, 2, at[frames - 1]);
        touch(automation, display, down, MotionEvent.ACTION_UP, 1, at[frames - 1]);

        System.out.print("phone:pinched");
        System.out.flush();
    }

    private static void touch(UiAutomation automation, int display, long down, int action, int pointers, float[] at)
            throws Exception {
        MotionEvent.PointerProperties[] properties = new MotionEvent.PointerProperties[pointers];
        MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[pointers];

        for (int i = 0; i < pointers; i++) {
            properties[i] = new MotionEvent.PointerProperties();
            properties[i].id = i;
            properties[i].toolType = MotionEvent.TOOL_TYPE_FINGER;

            coords[i] = new MotionEvent.PointerCoords();
            coords[i].x = at[i * 2];
            coords[i].y = at[i * 2 + 1];
            coords[i].pressure = 1;
            coords[i].size = 1;
        }

        MotionEvent event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, pointers, properties, coords,
                0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
        if (display != 0) {
            MotionEvent.class.getMethod("setDisplayId", int.class).invoke(event, display);
        }

        boolean taken = automation.injectInputEvent(event, true);
        event.recycle();
        if (!taken) {
            throw new IllegalStateException("the system refused " + MotionEvent.actionToString(action));
        }
    }

    // the window list is empty for a moment after the service connects
    private static List<AccessibilityWindowInfo> windows(UiAutomation automation) throws InterruptedException {
        List<AccessibilityWindowInfo> found = automation.getWindows();

        for (int i = 0; i < 30 && !anyRoot(found); i++) {
            Thread.sleep(100);
            found = automation.getWindows();
        }

        return found;
    }

    private static boolean anyRoot(List<AccessibilityWindowInfo> windows) {
        for (AccessibilityWindowInfo w : windows) {
            if (w.getRoot() != null) {
                return true;
            }
        }

        return false;
    }

    private static void node(AccessibilityNodeInfo n, Rect clip, StringBuilder out) {
        Rect r = new Rect();
        n.getBoundsInScreen(r);
        if (!r.intersect(clip)) {
            r.setEmpty();
        }

        out.append("<node");
        attr(out, "text", n.getText());
        attr(out, "resource-id", n.getViewIdResourceName());
        attr(out, "class", n.getClassName());
        attr(out, "package", n.getPackageName());
        attr(out, "content-desc", n.getContentDescription());
        attr(out, "hint", n.getHintText());
        attr(out, "clickable", String.valueOf(n.isClickable()));
        attr(out, "focused", String.valueOf(n.isFocused()));
        attr(out, "password", String.valueOf(n.isPassword()));
        attr(out, "bounds", bounds(r));
        out.append('>');

        for (int i = 0; i < n.getChildCount(); i++) {
            AccessibilityNodeInfo child = n.getChild(i);
            if (child != null && child.isVisibleToUser()) {
                node(child, r.isEmpty() ? clip : r, out);
            }
        }

        out.append("</node>");
    }

    private static void attr(StringBuilder out, String name, CharSequence value) {
        out.append(' ').append(name).append("=\"");

        if (value != null) {
            for (int i = 0; i < value.length(); i++) {
                char c = value.charAt(i);
                switch (c) {
                    case '&': out.append("&amp;"); break;
                    case '<': out.append("&lt;"); break;
                    case '>': out.append("&gt;"); break;
                    case '"': out.append("&quot;"); break;
                    case '\n': out.append("&#10;"); break;
                    default:
                        if (c >= 0x20 || c == '\t') {
                            out.append(c);
                        }
                }
            }
        }

        out.append('"');
    }

    private static String bounds(Rect r) {
        return "[" + r.left + "," + r.top + "][" + r.right + "," + r.bottom + "]";
    }

    private static String type(int type) {
        switch (type) {
            case AccessibilityWindowInfo.TYPE_APPLICATION: return "TYPE_APPLICATION";
            case AccessibilityWindowInfo.TYPE_INPUT_METHOD: return "TYPE_INPUT_METHOD";
            case AccessibilityWindowInfo.TYPE_SYSTEM: return "TYPE_SYSTEM";
            case AccessibilityWindowInfo.TYPE_ACCESSIBILITY_OVERLAY: return "TYPE_ACCESSIBILITY_OVERLAY";
            case AccessibilityWindowInfo.TYPE_SPLIT_SCREEN_DIVIDER: return "TYPE_SPLIT_SCREEN_DIVIDER";
            default: return "TYPE_" + type;
        }
    }

    private static Method hidden(String name, Class<?>... params) throws NoSuchMethodException {
        Method m = UiAutomation.class.getDeclaredMethod(name, params);
        m.setAccessible(true);
        return m;
    }
}
