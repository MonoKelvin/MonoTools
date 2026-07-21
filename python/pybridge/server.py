"""PyBridge Server - 通用 Python JSON-RPC 服务器

通过 stdin/stdout 与 Rust 侧通信，支持注册多个服务。
设计为独立模块：删除此目录不影响 Rust 侧功能（会自动降级）。

用法:
    python server.py

协议:
    每行一个 JSON-RPC 2.0 消息。
    请求: {"jsonrpc":"2.0","id":1,"method":"service.method","params":{...}}
    响应: {"jsonrpc":"2.0","id":1,"result":{...}}
    错误: {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"..."}}
"""

import sys
import json
import logging
import traceback
from typing import Dict, Callable, Any, Optional

# 配置日志到 stderr，避免污染 stdout (JSON-RPC 通道)
logging.basicConfig(
    level=logging.DEBUG,
    format="[%(levelname)s] %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("pybridge")


class PyBridgeServer:
    """PyBridge JSON-RPC 服务器

    支持注册服务方法，格式为 "service.method"。
    """

    def __init__(self):
        self._handlers: Dict[str, Callable] = {}
        self._services: Dict[str, dict] = {}
        self._register_builtin_methods()

    def _register_builtin_methods(self):
        """注册内置方法"""
        self.register("health_check", self._health_check)
        self.register("list_services", self._list_services)

    def register(self, method: str, handler: Callable):
        """注册一个方法处理器

        Args:
            method: 方法名，格式 "service.method" 或 "method"
            handler: 处理函数，接收 params dict，返回结果 dict
        """
        if method in self._handlers:
            logger.warning(f"方法 {method} 已存在，将被覆盖")
        self._handlers[method] = handler
        logger.info(f"注册方法: {method}")

    def register_service(self, name: str, version: str, description: str, methods: Dict[str, Callable]):
        """注册一个完整的服务

        Args:
            name: 服务名称
            version: 版本号
            description: 描述
            methods: 方法字典 {method_name: handler}
        """
        self._services[name] = {
            "name": name,
            "version": version,
            "description": description,
            "methods": list(methods.keys()),
        }

        for method_name, handler in methods.items():
            full_name = f"{name}.{method_name}"
            self.register(full_name, handler)

        logger.info(f"服务已注册: {name} v{version} ({len(methods)} 个方法)")

    def _health_check(self, params: dict) -> dict:
        """健康检查"""
        return {"status": "ok", "services": list(self._services.keys())}

    def _list_services(self, params: dict) -> dict:
        """列出所有已注册的服务"""
        return {"services": list(self._services.values())}

    def handle_request(self, request: dict) -> dict:
        """处理单个 JSON-RPC 请求"""
        request_id = request.get("id")
        method = request.get("method", "")
        params = request.get("params", {})

        if not method:
            return self._error_response(request_id, -32600, "Invalid Request: missing method")

        handler = self._handlers.get(method)
        if handler is None:
            return self._error_response(
                request_id, -32601, f"Method not found: {method}"
            )

        try:
            result = handler(params)
            return self._success_response(request_id, result)
        except Exception as e:
            logger.error(f"方法 {method} 执行失败: {e}\n{traceback.format_exc()}")
            return self._error_response(request_id, -32603, f"Internal error: {str(e)}")

    def _success_response(self, request_id: Optional[int], result: Any) -> dict:
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": result,
        }

    def _error_response(self, request_id: Optional[int], code: int, message: str) -> dict:
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": code,
                "message": message,
            },
        }

    def run(self):
        """运行服务器 (从 stdin 读取，输出到 stdout)"""
        logger.info("PyBridge server started")

        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                logger.error(f"解析 JSON 失败: {e}")
                continue

            response = self.handle_request(request)
            sys.stdout.write(json.dumps(response, ensure_ascii=False) + "\n")
            sys.stdout.flush()

        logger.info("PyBridge server stopped (stdin closed)")


def main():
    """主入口 - 创建服务器并注册所有可用服务"""
    server = PyBridgeServer()

    # 尝试注册 recommend 服务 (如果可用)
    try:
        import os
        import sys

        # 将当前目录加入 path，使 recommend 包可导入
        current_dir = os.path.dirname(os.path.abspath(__file__))
        parent_dir = os.path.dirname(current_dir)
        if parent_dir not in sys.path:
            sys.path.insert(0, parent_dir)

        from recommend.service import create_service

        rec_service = create_service()
        server.register_service(
            name="recommend",
            version="1.0.0",
            description="智能推荐服务",
            methods=rec_service,
        )
        logger.info("Recommend service loaded successfully")
    except ImportError as e:
        logger.warning(f"Recommend service not available: {e}")
    except Exception as e:
        logger.warning(f"Failed to load recommend service: {e}")

    # 启动服务器
    server.run()


if __name__ == "__main__":
    main()
