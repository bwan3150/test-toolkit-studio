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
import java.nio.ByteBuffer;

/**
 * 截图捕获工具
 * 使用 VirtualDisplay + ImageReader 从视频流获取截图
 */
public final class ScreenshotCapture {

    private ScreenshotCapture() {
        // not instantiable
    }

    /**
     * 截图并返回 PNG 格式的字节数组
     */
    public static byte[] captureScreenAsPng() {
        ImageReader imageReader = null;
        VirtualDisplay virtualDisplay = null;

        try {
            // 获取主显示器信息
            DisplayInfo displayInfo = ServiceManager.getDisplayManager().getDisplayInfo(0);
            if (displayInfo == null) {
                return null;
            }

            Size size = displayInfo.getSize();
            int width = size.getWidth();
            int height = size.getHeight();

            // 创建 ImageReader
            imageReader = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 2);
            Surface surface = imageReader.getSurface();

            // 创建 VirtualDisplay 捕获屏幕
            try {
                virtualDisplay = ServiceManager.getDisplayManager()
                        .createVirtualDisplay("tke-screenshot", width, height, 0, surface);
            } catch (Exception e) {
                return null;
            }

            // 等待一帧画面（增加等待时间）
            Thread.sleep(500);

            // 获取图像
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
        } finally {
            if (virtualDisplay != null) {
                virtualDisplay.release();
            }
            if (imageReader != null) {
                imageReader.close();
            }
        }
    }

    /**
     * 将 Image 转换为 Bitmap
     */
    private static Bitmap imageToBitmap(Image image, int width, int height) {
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

}
