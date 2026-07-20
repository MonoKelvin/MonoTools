# Python 推荐子模块

这是 MonoTools 的 Python 推荐子模块，通过 PyBridge 与 Rust 主进程通信。

## 目录结构

```
python/
├── pybridge/
│   └── server.py          # 通用 JSON-RPC 服务器
└── recommend/
    └── service.py         # 推荐服务 (TF-IDF + 序列模式)
```

## 安装依赖

```bash
pip install -r requirements.txt
```

## 独立运行测试

```bash
python -m recommend.service
```

## 算法说明

### 三层混合推荐

1. **内容推荐 (40%)**：基于 TF-IDF + 余弦相似度
   - 特征：应用名称、标签、类别
   - 上下文：前台应用标题、浏览器标签、查询词
   - 无 sklearn 时自动降级为简单字符串匹配

2. **序列模式 (30%)**：2-gram 转移统计
   - 捕捉"打开 A 后常打开 B"的使用模式
   - 从用户点击反馈中持续学习

3. **热度/频率 (30%)**：点击次数 + 启动次数
   - 对数归一化，避免高频应用垄断

### 在线学习

- 每次用户点击/启动都会更新点击计数和转移统计
- 状态持久化到 `~/.monotools/recommend/state.json`
- 冷启动时自动使用规则引擎（Rust 侧）作为保底

## 协议

通过 stdin/stdout 进行 JSON-RPC 2.0 通信，每行一个消息。
