package app.TestToolkit.TKE.AutoServer;

import app.TestToolkit.TKE.AutoServer.device.UITreeExporter;
import app.TestToolkit.TKE.AutoServer.util.Ln;

/**
 * Toolkit Engine AutoServer 主类
 *
 * 通过 app_process 运行在 Android 设备上，提供：
 * - UI 树导出（XML 格式）
 * - 截图（计划中）
 * - 自动化操作（计划中）
 *
 * 使用方法:
 * adb shell CLASSPATH=/data/local/tmp/tke-autoserver.jar app_process / app.TestToolkit.TKE.AutoServer.Server [command]
 *
 * 支持的命令：
 * - export-ui-tree : 导出当前 UI 树（XML 格式）
 * - version : 显示版本信息
 */
public final class Server {

    private static final String VERSION = "0.1.0";
    private static final String NAME = "Toolkit Engine AutoServer";

    private Server() {
        // not instantiable
    }

    public static void main(String... args) {
        // 设置异常处理
        Thread.setDefaultUncaughtExceptionHandler((t, e) -> {
            Ln.e("Exception on thread " + t, e);
            System.exit(1);
        });

        // 解析命令
        if (args.length == 0) {
            printUsage();
            System.exit(0);
            return;
        }

        String command = args[0];

        try {
            switch (command) {
                case "export-ui-tree":
                    exportUITree();
                    break;

                case "version":
                    printVersion();
                    break;

                case "help":
                case "--help":
                case "-h":
                    printUsage();
                    break;

                default:
                    Ln.e("Unknown command: " + command);
                    printUsage();
                    System.exit(1);
            }
        } catch (Exception e) {
            Ln.e("Failed to execute command: " + command, e);
            System.exit(1);
        }
    }

    /**
     * 导出 UI 树
     */
    private static void exportUITree() {
        Ln.i("Exporting UI tree...");
        String xmlTree = UITreeExporter.exportUITreeAsXml();
        System.out.println(xmlTree);
        System.out.flush();
        Ln.i("UI tree exported successfully");
    }

    /**
     * 打印版本信息
     */
    private static void printVersion() {
        System.out.println(NAME + " v" + VERSION);
    }

    /**
     * 打印使用说明
     */
    private static void printUsage() {
        System.out.println(NAME + " v" + VERSION);
        System.out.println();
        System.out.println("Usage:");
        System.out.println("  adb shell CLASSPATH=/data/local/tmp/tke-autoserver.jar app_process / app.TestToolkit.TKE.AutoServer.Server <command>");
        System.out.println();
        System.out.println("Commands:");
        System.out.println("  export-ui-tree    Export current UI hierarchy as XML");
        System.out.println("  version           Show version information");
        System.out.println("  help              Show this help message");
        System.out.println();
        System.out.println("Examples:");
        System.out.println("  # Export UI tree to file");
        System.out.println("  adb shell CLASSPATH=/data/local/tmp/tke-autoserver.jar app_process / app.TestToolkit.TKE.AutoServer.Server export-ui-tree > ui_tree.xml");
        System.out.println();
    }
}
