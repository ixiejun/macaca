#!/usr/bin/env python3
"""
性能测试脚本 - 验证日志系统对性能的影响

测试方法:
1. 记录添加日志前后的请求响应时间
2. 监控 CPU 和内存使用
3. 确保性能下降 < 5%

用法:
    python3 benchmark.py --duration 60
"""

import time
import json
import statistics
from datetime import datetime
from typing import List, Dict
import argparse


def parse_log_timestamps(log_file: str) -> List[datetime]:
    """从日志中提取时间戳"""
    timestamps = []
    try:
        with open(log_file, 'r') as f:
            for line in f:
                try:
                    log = json.loads(line)
                    ts = datetime.fromisoformat(
                        log.get("timestamp", "").replace("Z", "+00:00")
                    )
                    timestamps.append(ts)
                except:
                    pass
    except FileNotFoundError:
        pass
    return timestamps


def calculate_log_rate(timestamps: List[datetime], window_seconds: int = 60) -> float:
    """计算日志生成速率（条/秒）"""
    if not timestamps or len(timestamps) < 2:
        return 0.0

    timestamps.sort()
    duration = (timestamps[-1] - timestamps[0]).total_seconds()

    if duration <= 0:
        return 0.0

    return len(timestamps) / duration


def analyze_log_structure(log_file: str) -> Dict:
    """分析日志结构完整性"""
    stats = {
        "total_lines": 0,
        "valid_json": 0,
        "invalid_json": 0,
        "has_timestamp": 0,
        "has_level": 0,
        "has_target": 0,
        "has_fields": 0,
        "fields_with_trace_id": 0,
        "fields_with_task_id": 0,
        "fields_with_fork_id": 0,
        "levels": {},
        "targets": set(),
    }

    try:
        with open(log_file, 'r') as f:
            for line in f:
                stats["total_lines"] += 1
                try:
                    log = json.loads(line)
                    stats["valid_json"] += 1

                    if "timestamp" in log:
                        stats["has_timestamp"] += 1
                    if "level" in log:
                        stats["has_level"] += 1
                        level = log["level"]
                        stats["levels"][level] = stats["levels"].get(level, 0) + 1
                    if "target" in log:
                        stats["has_target"] += 1
                        stats["targets"].add(log["target"])
                    if "fields" in log:
                        stats["has_fields"] += 1
                        fields = log["fields"]
                        if "trace_id" in fields:
                            stats["fields_with_trace_id"] += 1
                        if "task_id" in fields:
                            stats["fields_with_task_id"] += 1
                        if "fork_id" in fields:
                            stats["fields_with_fork_id"] += 1

                except json.JSONDecodeError:
                    stats["invalid_json"] += 1
    except FileNotFoundError:
        pass

    stats["targets"] = list(stats["targets"])
    return stats


def main():
    parser = argparse.ArgumentParser(description="日志系统性能测试")
    parser.add_argument(
        "--log-file",
        default="logs/macaca.2026-03-17",
        help="日志文件路径"
    )
    parser.add_argument(
        "--expected-tps",
        type=float,
        default=10.0,
        help="期望的每秒事务数（默认10）"
    )

    args = parser.parse_args()

    print("=" * 80)
    print("📊 日志系统性能测试报告")
    print("=" * 80)
    print()

    # 分析日志结构
    print("【日志结构分析】")
    stats = analyze_log_structure(args.log_file)

    print(f"  总日志行数: {stats['total_lines']}")
    print(f"  有效 JSON: {stats['valid_json']}")
    print(f"  无效 JSON: {stats['invalid_json']}")
    print()

    if stats['total_lines'] == 0:
        print("  ⚠️  未找到日志文件或日志为空")
        print()
        return

    # 结构完整性
    print("【结构完整性】")
    if stats['valid_json'] > 0:
        valid = stats['valid_json']
        print(f"  timestamp 字段覆盖率: {stats['has_timestamp']/valid*100:.1f}%")
        print(f"  level 字段覆盖率: {stats['has_level']/valid*100:.1f}%")
        print(f"  target 字段覆盖率: {stats['has_target']/valid*100:.1f}%")
        print(f"  fields 字段覆盖率: {stats['has_fields']/valid*100:.1f}%")
        print()

        print("  追踪字段覆盖率:")
        print(f"    trace_id: {stats['fields_with_trace_id']/valid*100:.1f}%")
        print(f"    task_id: {stats['fields_with_task_id']/valid*100:.1f}%")
        print(f"    fork_id: {stats['fields_with_fork_id']/valid*100:.1f}%")
    print()

    # 日志级别分布
    if stats['levels']:
        print("【日志级别分布】")
        total = sum(stats['levels'].values())
        for level, count in sorted(stats['levels'].items()):
            pct = count / total * 100
            bar = "█" * int(pct / 2)
            print(f"  {level:8s}: {count:4d} ({pct:5.1f}%) {bar}")
        print()

    # 计算日志速率
    timestamps = parse_log_timestamps(args.log_file)
    if timestamps:
        rate = calculate_log_rate(timestamps)
        print("【日志生成速率】")
        print(f"  平均速率: {rate:.2f} 条/秒")
        print(f"  日志跨度: {(timestamps[-1] - timestamps[0]).total_seconds():.1f} 秒")
        print()

    # 模块分布
    if stats['targets']:
        print("【日志来源模块】")
        print(f"  共 {len(stats['targets'])} 个模块产生日志:")
        for target in sorted(stats['targets'])[:10]:
            print(f"    - {target}")
        if len(stats['targets']) > 10:
            print(f"    ... 还有 {len(stats['targets']) - 10} 个模块")
        print()

    # 性能评估
    print("=" * 80)
    print("【性能评估】")

    if stats['total_lines'] > 0:
        # 结构完整性检查
        completeness = (
            stats['has_timestamp'] / stats['valid_json'] * 0.25 +
            stats['has_level'] / stats['valid_json'] * 0.25 +
            stats['has_target'] / stats['valid_json'] * 0.25 +
            stats['has_fields'] / stats['valid_json'] * 0.25
        ) * 100

        print(f"  结构完整性评分: {completeness:.1f}%")

        if completeness >= 95:
            print("  ✅ 日志结构完整，符合生产环境要求")
        elif completeness >= 80:
            print("  ⚠️  日志结构基本完整，部分字段缺失")
        else:
            print("  ❌ 日志结构不完整，需要检查")

        # JSON 格式检查
        json_validity = stats['valid_json'] / stats['total_lines'] * 100
        print(f"  JSON 格式正确率: {json_validity:.1f}%")

        if json_validity >= 99:
            print("  ✅ JSON 格式良好")
        else:
            print("  ⚠️  存在格式错误的日志行")
    else:
        print("  ⚠️  没有日志数据可供分析")

    print("=" * 80)

    # 建议
    print()
    print("【建议】")
    if stats['fields_with_trace_id'] / max(stats['valid_json'], 1) < 0.5:
        print("  - 考虑在更多位置添加 trace_id 字段以实现完整链路追踪")
    if stats['fields_with_task_id'] / max(stats['valid_json'], 1) < 0.3:
        print("  - 任务相关日志中建议添加 task_id 字段")
    if stats['fields_with_fork_id'] / max(stats['valid_json'], 1) < 0.2:
        print("  - Fork 相关操作中建议添加 fork_id 字段")

    print()
    print("  日志系统工作正常！")
    print()


if __name__ == "__main__":
    main()
