package app.TestToolkit.TKE.AutoServer.device;

import android.view.View;
import android.view.ViewGroup;
import android.widget.Checkable;
import android.widget.TextView;

import app.TestToolkit.TKE.AutoServer.util.Ln;

import java.lang.reflect.Field;
import java.lang.reflect.Method;

/**
 * UI 树导出工具
 * 导出当前界面的 UI 结构（XML 格式），兼容 Android uiautomator dump 格式
 */
public final class UITreeExporter {

    private UITreeExporter() {
        // not instantiable
    }

    /**
     * 导出 XML 格式的 UI 树
     * 格式兼容 Android uiautomator dump
     */
    public static String exportUITreeAsXml() {
        try {
            // 通过反射获取 WindowManagerGlobal 实例
            Class<?> windowManagerGlobalClass = Class.forName("android.view.WindowManagerGlobal");
            Method getInstanceMethod = windowManagerGlobalClass.getMethod("getInstance");
            Object wmgInstance = getInstanceMethod.invoke(null);

            // 获取 mRoots 字段（所有 ViewRootImpl 实例）
            Field mRootsField = windowManagerGlobalClass.getDeclaredField("mRoots");
            mRootsField.setAccessible(true);
            @SuppressWarnings("unchecked")
            java.util.List<?> viewRootImpls = (java.util.List<?>) mRootsField.get(wmgInstance);

            StringBuilder xmlBuilder = new StringBuilder();
            xmlBuilder.append("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
            xmlBuilder.append("<hierarchy rotation=\"0\">\n");

            // 遍历所有根 View
            if (viewRootImpls != null) {
                for (Object viewRootImpl : viewRootImpls) {
                    try {
                        Method getViewMethod = viewRootImpl.getClass().getMethod("getView");
                        View rootView = (View) getViewMethod.invoke(viewRootImpl);

                        if (rootView != null) {
                            traverseViewToXml(rootView, xmlBuilder, 1);
                        }
                    } catch (Exception e) {
                        Ln.w("Failed to process ViewRootImpl", e);
                    }
                }
            }

            xmlBuilder.append("</hierarchy>");
            return xmlBuilder.toString();

        } catch (Exception e) {
            Ln.e("Failed to export UI tree as XML", e);
            return "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<error>" + e.getMessage() + "</error>";
        }
    }

    /**
     * 递归遍历 View 树，转换为 XML
     */
    private static void traverseViewToXml(View view, StringBuilder sb, int depth) {
        String indent = "  ".repeat(depth);

        // 开始标签
        sb.append(indent).append("<node");

        // class 属性
        sb.append(" class=\"").append(view.getClass().getName()).append("\"");

        // bounds 属性 [x1,y1][x2,y2]
        int[] location = new int[2];
        view.getLocationOnScreen(location);
        sb.append(" bounds=\"[")
          .append(location[0]).append(",").append(location[1])
          .append("][")
          .append(location[0] + view.getWidth()).append(",")
          .append(location[1] + view.getHeight())
          .append("]\"");

        // text 属性
        if (view instanceof TextView) {
            CharSequence text = ((TextView) view).getText();
            if (text != null && text.length() > 0) {
                sb.append(" text=\"").append(escapeXml(text.toString())).append("\"");
            }
        }

        // hint 属性
        if (view instanceof TextView) {
            CharSequence hint = ((TextView) view).getHint();
            if (hint != null && hint.length() > 0) {
                sb.append(" hint=\"").append(escapeXml(hint.toString())).append("\"");
            }
        }

        // content-desc 属性
        CharSequence contentDesc = view.getContentDescription();
        if (contentDesc != null && contentDesc.length() > 0) {
            sb.append(" content-desc=\"").append(escapeXml(contentDesc.toString())).append("\"");
        }

        // resource-id 属性
        try {
            int id = view.getId();
            if (id != View.NO_ID) {
                String resourceName = view.getResources().getResourceName(id);
                sb.append(" resource-id=\"").append(escapeXml(resourceName)).append("\"");
            }
        } catch (Exception e) {
            // ignore - 有些 View 没有 resource-id
        }

        // 布尔属性
        sb.append(" clickable=\"").append(view.isClickable()).append("\"");
        sb.append(" focusable=\"").append(view.isFocusable()).append("\"");
        sb.append(" focused=\"").append(view.isFocused()).append("\"");
        sb.append(" scrollable=\"").append(view.isScrollContainer()).append("\"");
        sb.append(" enabled=\"").append(view.isEnabled()).append("\"");
        sb.append(" selected=\"").append(view.isSelected()).append("\"");

        // checkable 和 checked 属性
        boolean checkable = view instanceof Checkable;
        sb.append(" checkable=\"").append(checkable).append("\"");
        if (checkable) {
            sb.append(" checked=\"").append(((Checkable) view).isChecked()).append("\"");
        } else {
            sb.append(" checked=\"false\"");
        }

        // 子节点
        if (view instanceof ViewGroup) {
            ViewGroup viewGroup = (ViewGroup) view;
            int childCount = viewGroup.getChildCount();
            if (childCount > 0) {
                sb.append(">\n");
                for (int i = 0; i < childCount; i++) {
                    try {
                        View child = viewGroup.getChildAt(i);
                        if (child != null) {
                            traverseViewToXml(child, sb, depth + 1);
                        }
                    } catch (Exception e) {
                        Ln.w("Failed to process child view at index " + i, e);
                    }
                }
                sb.append(indent).append("</node>\n");
            } else {
                sb.append(" />\n");
            }
        } else {
            sb.append(" />\n");
        }
    }

    /**
     * XML 转义
     */
    private static String escapeXml(String str) {
        return str.replace("&", "&amp;")
                  .replace("<", "&lt;")
                  .replace(">", "&gt;")
                  .replace("\"", "&quot;")
                  .replace("'", "&apos;");
    }
}
