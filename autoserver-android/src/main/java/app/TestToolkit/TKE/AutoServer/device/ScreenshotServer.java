package app.TestToolkit.TKE.AutoServer.device;

import android.graphics.Bitmap;
import android.graphics.PixelFormat;
import android.hardware.display.VirtualDisplay;
import android.media.Image;
import android.media.ImageReader;
import android.view.Surface;

import app.TestToolkit.TKE.AutoServer.util.Ln;
import app.TestToolkit.TKE.AutoServer.wrappers.ServiceManager;

import java.io.ByteArrayOutputStream;
import java.io.OutputStream;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;

/**
 * 持续运行的截图服务器
 * 保持 VirtualDisplay 一直打开，接收截图请求时立刻从视频流中获取当前帧
 */
public final class ScreenshotServer {

    private static final int PORT = 8765; // 截图服务器端口

    private ImageReader imageReader;
    private VirtualDisplay virtualDisplay;
    private int width;
    private int height;
    private volatile boolean running = true;

    public ScreenshotServer() {
    }

    /**
     * 启动截图服务器
     */
    public void start() {
        try {
            // 初始化 VirtualDisplay
            if (!initVirtualDisplay()) {
                Ln.e("Failed to initialize virtual display");
                return;
            }

            Ln.i("Screenshot server started on port " + PORT);
            Ln.i("Display size: " + width + "x" + height);

            // 启动 socket 服务器
            ServerSocket serverSocket = new ServerSocket(PORT);

            while (running) {
                try {
                    Socket clientSocket = serverSocket.accept();
                    handleClient(clientSocket);
                } catch (Exception e) {
                    if (running) {
                        Ln.e("Error handling client", e);
                    }
                }
            }

            serverSocket.close();

        } catch (Exception e) {
            Ln.e("Screenshot server error", e);
        } finally {
            cleanup();
        }
    }

    /**
     * 初始化 VirtualDisplay
     */
    private boolean initVirtualDisplay() {
        try {
            // 获取主显示器信息
            DisplayInfo displayInfo = ServiceManager.getDisplayManager().getDisplayInfo(0);
            if (displayInfo == null) {
                return false;
            }

            Size size = displayInfo.getSize();
            this.width = size.getWidth();
            this.height = size.getHeight();

            // 创建 ImageReader
            imageReader = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 3);
            Surface surface = imageReader.getSurface();

            // 创建 VirtualDisplay 并保持运行
            virtualDisplay = ServiceManager.getDisplayManager()
                    .createVirtualDisplay("tke-screenshot-server", width, height, 0, surface);

            // 等待第一帧
            Thread.sleep(200);

            return true;
        } catch (Exception e) {
            Ln.e("Failed to init virtual display", e);
            return false;
        }
    }

    /**
     * 处理客户端截图请求
     */
    private void handleClient(Socket clientSocket) {
        try {
            // 立刻从视频流获取当前帧
            byte[] pngData = captureCurrentFrame();

            if (pngData != null) {
                // 发送 PNG 数据给客户端
                OutputStream out = clientSocket.getOutputStream();
                out.write(pngData);
                out.flush();
            }

            clientSocket.close();
        } catch (Exception e) {
            Ln.e("Error handling client request", e);
        }
    }

    /**
     * 从当前视频流中捕获一帧
     */
    private byte[] captureCurrentFrame() {
        try {
            // 直接获取最新的图像（视频流一直在运行，所以有数据）
            Image image = imageReader.acquireLatestImage();
            if (image == null) {
                return null;
            }

            // 转换为 Bitmap
            Bitmap bitmap = imageToBitmap(image, width, height);
            image.close();

            if (bitmap == null) {
                return null;
            }

            // 转换为 PNG
            ByteArrayOutputStream baos = new ByteArrayOutputStream();
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, baos);
            bitmap.recycle();

            return baos.toByteArray();

        } catch (Exception e) {
            return null;
        }
    }

    /**
     * 将 Image 转换为 Bitmap
     */
    private Bitmap imageToBitmap(Image image, int width, int height) {
        try {
            Image.Plane[] planes = image.getPlanes();
            ByteBuffer buffer = planes[0].getBuffer();
            int pixelStride = planes[0].getPixelStride();
            int rowStride = planes[0].getRowStride();
            int rowPadding = rowStride - pixelStride * width;

            Bitmap bitmap = Bitmap.createBitmap(
                width + rowPadding / pixelStride,
                height,
                Bitmap.Config.ARGB_8888
            );
            bitmap.copyPixelsFromBuffer(buffer);

            // 如果有 padding，需要裁剪
            if (rowPadding != 0) {
                Bitmap croppedBitmap = Bitmap.createBitmap(bitmap, 0, 0, width, height);
                bitmap.recycle();
                return croppedBitmap;
            }

            return bitmap;
        } catch (Exception e) {
            return null;
        }
    }

    /**
     * 停止服务器并清理资源
     */
    public void stop() {
        running = false;
    }

    /**
     * 清理资源
     */
    private void cleanup() {
        if (virtualDisplay != null) {
            virtualDisplay.release();
            virtualDisplay = null;
        }
        if (imageReader != null) {
            imageReader.close();
            imageReader = null;
        }
    }
}
