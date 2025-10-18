#!/usr/bin/env python3
"""
OpenCV 模板匹配工具
用于在截图中查找 UI 元素模板，支持多尺度匹配和非极大值抑制
"""

import cv2
import numpy as np
import json
import sys
from pathlib import Path

__version__ = "0.4.2"


def non_max_suppression(boxes, overlap_thresh=0.5):
    """
    非极大值抑制 (NMS) - 去除重叠的检测框

    Args:
        boxes: numpy array of shape (N, 4), 每行为 [x, y, w, h]
        overlap_thresh: IOU 阈值

    Returns:
        去重后的检测框列表
    """
    if len(boxes) == 0:
        return []

    boxes = np.array(boxes)

    # 获取坐标
    x1 = boxes[:, 0]
    y1 = boxes[:, 1]
    x2 = boxes[:, 0] + boxes[:, 2]
    y2 = boxes[:, 1] + boxes[:, 3]

    # 计算面积
    areas = boxes[:, 2] * boxes[:, 3]

    # 按 y 坐标排序
    idxs = np.argsort(y1)

    pick = []

    while len(idxs) > 0:
        # 取出最后一个索引
        last = len(idxs) - 1
        i = idxs[last]
        pick.append(i)

        # 计算当前框与其他框的交集
        xx1 = np.maximum(x1[i], x1[idxs[:last]])
        yy1 = np.maximum(y1[i], y1[idxs[:last]])
        xx2 = np.minimum(x2[i], x2[idxs[:last]])
        yy2 = np.minimum(y2[i], y2[idxs[:last]])

        # 计算交集面积
        w = np.maximum(0, xx2 - xx1)
        h = np.maximum(0, yy2 - yy1)
        overlap = (w * h) / areas[idxs[:last]]

        # 删除重叠度高的框
        idxs = np.delete(idxs, np.concatenate(([last], np.where(overlap > overlap_thresh)[0])))

    return boxes[pick].tolist()


def template_match(screenshot_path, template_path, threshold=0.75, match_index=0):
    """
    多尺度模板匹配

    Args:
        screenshot_path: 截图路径
        template_path: 模板图片路径
        threshold: 匹配阈值 (0.0 - 1.0)
        match_index: 返回第几个匹配结果 (0 为第一个)

    Returns:
        JSON 格式的匹配结果
    """
    # 验证文件存在
    if not Path(screenshot_path).exists():
        return {
            "success": False,
            "error": f"截图文件不存在: {screenshot_path}"
        }

    if not Path(template_path).exists():
        return {
            "success": False,
            "error": f"模板文件不存在: {template_path}"
        }

    # 加载截图（灰度）
    screenshot = cv2.imread(screenshot_path, cv2.IMREAD_GRAYSCALE)
    if screenshot is None:
        return {
            "success": False,
            "error": f"无法加载截图: {screenshot_path}"
        }

    # 加载模板（灰度）
    template = cv2.imread(template_path, cv2.IMREAD_GRAYSCALE)
    if template is None:
        return {
            "success": False,
            "error": f"无法加载模板: {template_path}"
        }

    template_h, template_w = template.shape[:2]
    screenshot_h, screenshot_w = screenshot.shape[:2]

    all_matches = []

    # 多尺度匹配：从 0.5 到 1.5 倍，20 个尺度
    for i in range(20):
        scale = 0.5 + (1.0 / 19.0) * i

        # 缩放模板
        resized_w = int(template_w * scale)
        resized_h = int(template_h * scale)

        # 如果缩放后的模板比截图大，跳过
        if resized_w > screenshot_w or resized_h > screenshot_h:
            continue

        resized_template = cv2.resize(template, (resized_w, resized_h), interpolation=cv2.INTER_LINEAR)

        # 模板匹配 (归一化互相关系数)
        result = cv2.matchTemplate(screenshot, resized_template, cv2.TM_CCOEFF_NORMED)

        # 找到所有超过阈值的位置
        locations = np.where(result >= threshold)

        # 收集匹配点
        for pt in zip(*locations[::-1]):  # pt = (x, y)
            x, y = pt
            all_matches.append([x, y, resized_w, resized_h])

    # 如果没有找到匹配
    if not all_matches:
        return {
            "success": False,
            "error": f"未找到匹配点 (阈值={threshold})"
        }

    # 非极大值抑制（去重）
    unique_matches = non_max_suppression(np.array(all_matches), overlap_thresh=0.5)

    # 按 y 坐标排序
    unique_matches = sorted(unique_matches, key=lambda m: m[1])

    # 检查 match_index 是否有效
    if match_index >= len(unique_matches):
        match_index = len(unique_matches) - 1

    # 获取目标匹配
    target_match = unique_matches[match_index]
    x, y, w, h = target_match

    # 计算中心点
    center_x = int(x + w / 2)
    center_y = int(y + h / 2)

    # 返回结果
    return {
        "success": True,
        "x": center_x,
        "y": center_y,
        "width": w,
        "height": h,
        "matches_count": len(unique_matches)
    }


def main():
    """命令行入口"""
    # 处理 --version 参数
    if len(sys.argv) == 2 and sys.argv[1] in ['--version', '-v']:
        print(f"tke-opencv {__version__}")
        sys.exit(0)

    if len(sys.argv) < 3:
        result = {
            "success": False,
            "error": "用法: opencv_matcher.py <screenshot_path> <template_path> [threshold] [match_index]"
        }
        print(json.dumps(result, ensure_ascii=False))
        sys.exit(1)

    screenshot_path = sys.argv[1]
    template_path = sys.argv[2]
    threshold = float(sys.argv[3]) if len(sys.argv) > 3 else 0.75
    match_index = int(sys.argv[4]) if len(sys.argv) > 4 else 0

    try:
        result = template_match(screenshot_path, template_path, threshold, match_index)
        print(json.dumps(result, ensure_ascii=False))

        # 如果失败，返回非零退出码
        sys.exit(0 if result["success"] else 1)

    except Exception as e:
        result = {
            "success": False,
            "error": f"发生异常: {str(e)}"
        }
        print(json.dumps(result, ensure_ascii=False))
        sys.exit(1)


if __name__ == "__main__":
    main()
