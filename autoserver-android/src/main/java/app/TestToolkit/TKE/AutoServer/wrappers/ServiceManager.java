package app.TestToolkit.TKE.AutoServer.wrappers;

import android.annotation.SuppressLint;
import android.os.IBinder;
import android.os.IInterface;

import java.lang.reflect.Method;

/**
 * 系统服务管理器
 * 通过反射访问 Android 隐藏的系统服务
 */
@SuppressLint("PrivateApi,DiscouragedPrivateApi")
public final class ServiceManager {

    private static final Method GET_SERVICE_METHOD;

    static {
        try {
            GET_SERVICE_METHOD = Class.forName("android.os.ServiceManager")
                .getDeclaredMethod("getService", String.class);
        } catch (Exception e) {
            throw new AssertionError(e);
        }
    }

    private ServiceManager() {
        // not instantiable
    }

    /**
     * 获取系统服务
     *
     * @param service 服务名称（如 "window"）
     * @param type 服务接口类型（如 "android.view.IWindowManager"）
     * @return 服务接口实例
     */
    public static IInterface getService(String service, String type) {
        try {
            IBinder binder = (IBinder) GET_SERVICE_METHOD.invoke(null, service);
            Method asInterfaceMethod = Class.forName(type + "$Stub")
                .getMethod("asInterface", IBinder.class);
            return (IInterface) asInterfaceMethod.invoke(null, binder);
        } catch (Exception e) {
            throw new AssertionError(e);
        }
    }
}
