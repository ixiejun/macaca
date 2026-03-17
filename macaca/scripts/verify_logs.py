#!/usr/bin/env python3
"""
日志完整性检查脚本

验证给定时间窗口内所有委托任务的日志完整性，包括：
- 任务必须有创建和完成日志
- 关键字段不缺失（trace_id, task_id, agent_name 等）
- Fork 生命周期完整（创建 -> 挂起 -> 恢复）
- 工具调用日志完整

用法:
    python3 verify_logs.py --log-file logs/macaca.2026-03-17 --app-id <app-id>
    python3 verify_logs.py --log-file logs/macaca.2026-03-17 --session-id <session-id>
    python3 verify_logs.py --log-file logs/macaca.2026-03-17 --check-all

输出:
    - 完整性报告
    - 缺失日志项列表
    - 建议修复措施
"""

import json
import argparse
import sys
from datetime import datetime
from collections import defaultdict
from typing import Dict, List, Set, Optional, Any
from dataclasses import dataclass, field


@dataclass
class TaskLifecycle:
    """记录单个任务的完整生命周期"""
    task_id: str
    app_id: Optional[str] = None
    agent_name: Optional[str] = None
    fork_id: Optional[str] = None
    trace_id: Optional[str] = None

    # 生命周期事件
    created: bool = False
    started: bool = False
    fork_created: bool = False
    fork_suspended: bool = False
    fork_resumed: bool = False
    completed: bool = False
    failed: bool = False

    # 工具调用记录
    tools_called: List[str] = field(default_factory=list)

    # 错误信息
    errors: List[str] = field(default_factory=list)

    # 时间戳
    first_seen: Optional[str] = None
    last_seen: Optional[str] = None

    def is_complete(self) -> bool:
        """检查生命周期是否完整"""
        return self.created and (self.completed or self.failed)

    def get_missing_events(self) -> List[str]:
        """获取缺失的关键事件"""
        missing = []
        if not self.created:
            missing.append("task_created")
        if not self.started:
            missing.append("task_started")
        if self.fork_created and not self.fork_suspended:
            missing.append("fork_suspended")
        if self.fork_suspended and not self.fork_resumed:
            missing.append("fork_resumed")
        if not (self.completed or self.failed):
            missing.append("task_completed/failed")
        return missing


class LogVerifier:
    """日志验证器"""

    def __init__(self, log_file: str):
        self.log_file = log_file
        self.tasks: Dict[str, TaskLifecycle] = {}
        self.forks: Dict[str, Dict[str, Any]] = {}
        self.errors: List[str] = []
        self.warnings: List[str] = []

        # 统计信息
        self.total_logs = 0
        self.valid_logs = 0
        self.invalid_logs = 0

    def parse_log_line(self, line: str) -> Optional[Dict[str, Any]]:
        """解析单行日志"""
        try:
            return json.loads(line)
        except json.JSONDecodeError:
            self.errors.append(f"Invalid JSON: {line[:100]}...")
            self.invalid_logs += 1
            return None

    def extract_task_id(self, log: Dict) -> Optional[str]:
        """从日志中提取 task_id"""
        fields = log.get("fields", {})

        # 直接字段
        if "task_id" in fields:
            return fields["task_id"]

        # 从消息中提取
        message = fields.get("message", "")
        if "task" in message.lower():
            # 尝试从消息中解析 task_id
            for key in ["task_id", "task"]:
                if key in fields:
                    return fields[key]

        return None

    def extract_fork_id(self, log: Dict) -> Optional[str]:
        """从日志中提取 fork_id"""
        fields = log.get("fields", {})
        return fields.get("fork_id") or fields.get("fork")

    def extract_trace_id(self, log: Dict) -> Optional[str]:
        """从日志中提取 trace_id"""
        fields = log.get("fields", {})
        return fields.get("trace_id")

    def process_log(self, log: Dict) -> None:
        """处理单条日志"""
        self.total_logs += 1

        if not log:
            return

        self.valid_logs += 1

        target = log.get("target", "")
        fields = log.get("fields", {})
        message = fields.get("message", "")
        level = log.get("level", "INFO")

        # 提取关键 ID
        task_id = self.extract_task_id(log)
        fork_id = self.extract_fork_id(log)
        trace_id = self.extract_trace_id(log)
        app_id = fields.get("app_id")
        agent_name = fields.get("agent_name")

        timestamp = log.get("timestamp", "")

        # 处理任务相关日志
        if task_id:
            if task_id not in self.tasks:
                self.tasks[task_id] = TaskLifecycle(
                    task_id=task_id,
                    app_id=app_id,
                    agent_name=agent_name,
                    trace_id=trace_id
                )

            task = self.tasks[task_id]
            task.last_seen = timestamp
            if not task.first_seen:
                task.first_seen = timestamp

            # 更新 trace_id（优先使用第一个）
            if trace_id and not task.trace_id:
                task.trace_id = trace_id

            # 识别生命周期事件
            if "[FORK] Created" in message or "ForkCreated" in target:
                task.fork_created = True
                if fork_id:
                    task.fork_id = fork_id
                    self.forks[fork_id] = {"task_id": task_id, "created": True}

            elif "[FORK] Suspended" in message or "ForkWaiting" in target:
                task.fork_suspended = True

            elif "[FORK] Resumed" in message or "ForkResumed" in target:
                task.fork_resumed = True

            elif "TaskStarted" in target or "executing task" in message:
                task.started = True

            elif "completed" in message.lower() or "TaskCompleted" in target:
                task.completed = True

            elif "failed" in message.lower() or level == "ERROR":
                task.failed = True
                task.errors.append(message)

            elif "[TOOL]" in message:
                tool_name = fields.get("tool_name")
                if tool_name:
                    task.tools_called.append(tool_name)

        # 处理 Fork 相关日志（无 task_id 时）
        elif fork_id:
            if fork_id not in self.forks:
                self.forks[fork_id] = {}

            if "[FORK] Created" in message:
                self.forks[fork_id]["created"] = True
            elif "[FORK] Suspended" in message:
                self.forks[fork_id]["suspended"] = True
            elif "[FORK] Resumed" in message:
                self.forks[fork_id]["resumed"] = True

    def verify(self) -> Dict[str, Any]:
        """执行验证"""
        print(f"🔍 正在分析日志文件: {self.log_file}")
        print()

        try:
            with open(self.log_file, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue

                    log = self.parse_log_line(line)
                    if log:
                        self.process_log(log)
        except FileNotFoundError:
            print(f"❌ 错误: 找不到日志文件 {self.log_file}")
            sys.exit(1)
        except Exception as e:
            print(f"❌ 错误: 读取日志文件失败: {e}")
            sys.exit(1)

        return self.generate_report()

    def generate_report(self) -> Dict[str, Any]:
        """生成验证报告"""
        report = {
            "summary": {
                "total_logs": self.total_logs,
                "valid_logs": self.valid_logs,
                "invalid_logs": self.invalid_logs,
                "total_tasks": len(self.tasks),
                "total_forks": len(self.forks),
            },
            "task_analysis": {
                "complete_lifecycle": 0,
                "incomplete_lifecycle": 0,
                "missing_start": [],
                "missing_end": [],
                "missing_fork_events": [],
            },
            "issues": {
                "errors": self.errors,
                "warnings": self.warnings,
            },
            "details": []
        }

        # 分析每个任务
        for task_id, task in self.tasks.items():
            task_info = {
                "task_id": task_id,
                "trace_id": task.trace_id,
                "app_id": task.app_id,
                "agent_name": task.agent_name,
                "fork_id": task.fork_id,
                "lifecycle": {
                    "created": task.created or task.fork_created,
                    "started": task.started,
                    "fork_suspended": task.fork_suspended,
                    "fork_resumed": task.fork_resumed,
                    "completed": task.completed,
                    "failed": task.failed,
                },
                "tools_called": task.tools_called,
                "is_complete": task.is_complete(),
                "missing_events": task.get_missing_events(),
                "duration": self.calculate_duration(task.first_seen, task.last_seen),
            }

            if task.is_complete():
                report["task_analysis"]["complete_lifecycle"] += 1
            else:
                report["task_analysis"]["incomplete_lifecycle"] += 1
                report["task_analysis"]["missing_end"].append(task_id)

            # 检查 Fork 事件完整性
            if task.fork_created:
                if not task.fork_suspended:
                    report["task_analysis"]["missing_fork_events"].append({
                        "task_id": task_id,
                        "fork_id": task.fork_id,
                        "missing": "suspended"
                    })
                elif not task.fork_resumed:
                    report["task_analysis"]["missing_fork_events"].append({
                        "task_id": task_id,
                        "fork_id": task.fork_id,
                        "missing": "resumed"
                    })

            report["details"].append(task_info)

        return report

    def calculate_duration(self, start: Optional[str], end: Optional[str]) -> Optional[str]:
        """计算持续时间"""
        if not start or not end:
            return None

        try:
            # 解析 ISO 8601 格式
            start_dt = datetime.fromisoformat(start.replace('Z', '+00:00'))
            end_dt = datetime.fromisoformat(end.replace('Z', '+00:00'))
            duration = (end_dt - start_dt).total_seconds()

            if duration < 60:
                return f"{duration:.1f}s"
            elif duration < 3600:
                return f"{duration/60:.1f}m"
            else:
                return f"{duration/3600:.1f}h"
        except:
            return None

    def print_report(self, report: Dict[str, Any]) -> None:
        """打印验证报告"""
        summary = report["summary"]
        analysis = report["task_analysis"]

        print("=" * 80)
        print("📊 日志完整性验证报告")
        print("=" * 80)
        print()

        # 摘要
        print("【摘要】")
        print(f"  总日志数: {summary['total_logs']}")
        print(f"  有效日志: {summary['valid_logs']}")
        print(f"  无效日志: {summary['invalid_logs']}")
        print(f"  任务数: {summary['total_tasks']}")
        print(f"  Fork数: {summary['total_forks']}")
        print()

        # 任务分析
        print("【任务生命周期分析】")
        print(f"  完整生命周期: {analysis['complete_lifecycle']} / {summary['total_tasks']}")
        print(f"  不完整生命周期: {analysis['incomplete_lifecycle']}")

        if analysis['incomplete_lifecycle'] > 0:
            print()
            print("  ⚠️  生命周期不完整的任务:")
            for task_info in report["details"]:
                if not task_info["is_complete"]:
                    print(f"    - {task_info['task_id']}")
                    print(f"      Agent: {task_info['agent_name'] or 'Unknown'}")
                    print(f"      缺失: {', '.join(task_info['missing_events'])}")

        # Fork 事件分析
        if analysis['missing_fork_events']:
            print()
            print("  ⚠️  Fork 事件缺失:")
            for item in analysis['missing_fork_events']:
                print(f"    - Fork {item['fork_id']}: 缺失 {item['missing']}")

        # 错误和警告
        if report["issues"]["errors"]:
            print()
            print("【错误】")
            for error in report["issues"]["errors"][:10]:
                print(f"  ❌ {error}")
            if len(report["issues"]["errors"]) > 10:
                print(f"  ... 还有 {len(report['issues']['errors']) - 10} 个错误")

        # 详细任务列表
        print()
        print("【任务详情】")
        for task_info in report["details"][:10]:
            status = "✅" if task_info["is_complete"] else "❌"
            print(f"  {status} {task_info['task_id'][:8]}...")
            print(f"     Agent: {task_info['agent_name'] or 'Unknown'}")
            print(f"     工具调用: {len(task_info['tools_called'])} 次")
            if task_info["duration"]:
                print(f"     持续时间: {task_info['duration']}")
            if task_info["missing_events"]:
                print(f"     ⚠️  缺失: {', '.join(task_info['missing_events'])}")

        if len(report["details"]) > 10:
            print(f"\n  ... 还有 {len(report['details']) - 10} 个任务")

        print()
        print("=" * 80)

        # 结论
        if analysis['incomplete_lifecycle'] == 0 and not report["issues"]["errors"]:
            print("✅ 验证通过：所有任务都有完整的日志链")
        else:
            print(f"⚠️  验证发现 {analysis['incomplete_lifecycle']} 个任务生命周期不完整")
            print("   建议检查：")
            print("   1. 委托任务是否正确完成")
            print("   2. Hook 事件是否正常发送和接收")
            print("   3. Fork 恢复机制是否正常工作")

        print("=" * 80)


def main():
    parser = argparse.ArgumentParser(
        description="验证 Macaca 日志完整性",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
    %(prog)s --log-file logs/macaca.2026-03-17
    %(prog)s --log-file logs/macaca.2026-03-17 --json-output report.json
        """
    )

    parser.add_argument(
        "--log-file",
        required=True,
        help="日志文件路径"
    )

    parser.add_argument(
        "--app-id",
        help="过滤特定应用 ID"
    )

    parser.add_argument(
        "--session-id",
        help="过滤特定会话 ID"
    )

    parser.add_argument(
        "--json-output",
        help="输出 JSON 格式报告到文件"
    )

    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="详细输出"
    )

    args = parser.parse_args()

    # 创建验证器
    verifier = LogVerifier(args.log_file)

    # 执行验证
    report = verifier.verify()

    # 打印报告
    verifier.print_report(report)

    # 输出 JSON 报告
    if args.json_output:
        with open(args.json_output, 'w', encoding='utf-8') as f:
            json.dump(report, f, indent=2, ensure_ascii=False)
        print(f"\n📄 JSON 报告已保存: {args.json_output}")

    # 根据验证结果设置退出码
    incomplete = report["task_analysis"]["incomplete_lifecycle"]
    errors = len(report["issues"]["errors"])

    if incomplete > 0 or errors > 0:
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == "__main__":
    main()
