"""推荐服务 - 基于内容 + 老虎机 + 序列模式的混合推荐"""

import json
import os
import logging
from typing import List, Dict, Any, Tuple

logger = logging.getLogger("pybridge.recommend")

# 可选依赖 - 如果没有就用简单实现
try:
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.metrics.pairwise import cosine_similarity
    import numpy as np

    HAS_SKLEARN = True
    logger.info("scikit-learn available, using TF-IDF content recommendation")
except ImportError:
    HAS_SKLEARN = False
    logger.warning("scikit-learn not available, using simple content matching")

try:
    # lightrl LinUCB - 如果没有就跳过老虎机部分
    HAS_LIGHTRL = False
    logger.warning("lightrl not available, bandit recommendation disabled")
except ImportError:
    HAS_LIGHTRL = False


class RecommendService:
    """推荐服务 - 三层混合推荐"""

    def __init__(self):
        self.items: Dict[str, Dict[str, Any]] = {}
        self.item_ids: List[str] = []

        # TF-IDF 相关
        self._tfidf_vectorizer = None
        self._tfidf_matrix = None

        # 序列模式统计
        self._transition_2gram: Dict[Tuple[str, str], int] = {}
        self._click_counts: Dict[str, int] = {}

        # 模型持久化路径
        self._data_dir = os.path.join(
            os.path.expanduser("~"), ".monotools", "recommend"
        )
        os.makedirs(self._data_dir, exist_ok=True)

        self._initialized = False

    # ===== 公共 API =====

    def initialize(self, params: dict) -> dict:
        """初始化推荐服务，传入所有候选应用"""
        items = params.get("items", [])
        self._load_items(items)
        self._build_content_index()
        self._load_state()
        self._initialized = True
        logger.info(f"Recommend initialized with {len(self.items)} items")
        return {"success": True, "item_count": len(self.items)}

    def get_scores(self, params: dict) -> dict:
        """获取推荐分数"""
        if not self._initialized:
            return {"scores": []}

        items = params.get("items", [])
        context = params.get("context", {})

        scores = []
        for item in items:
            item_id = item.get("id", "")
            score = self._compute_score(item, context)
            scores.append({"id": item_id, "score": score})

        # 按分数降序
        scores.sort(key=lambda x: x["score"], reverse=True)

        return {"scores": scores}

    def report_feedback(self, params: dict) -> dict:
        """上报用户反馈"""
        feedback = params.get("feedback", {})
        item_id = feedback.get("item_id", "")
        feedback_type = feedback.get("feedback_type", "")

        if feedback_type in ("click", "pin"):
            self._click_counts[item_id] = self._click_counts.get(item_id, 0) + 1

            # 更新序列模式 (上下文中的 recent_app_ids → 点击的 item)
            context = feedback.get("context", {})
            recent = context.get("recent_app_ids", [])
            if recent:
                last = recent[-1]
                key = (last, item_id)
                self._transition_2gram[key] = self._transition_2gram.get(key, 0) + 1

            self._save_state()

        return {"success": True}

    # ===== 内部方法 =====

    def _load_items(self, items: List[dict]):
        """加载候选项目"""
        self.items = {}
        self.item_ids = []
        for item in items:
            item_id = item.get("id", "")
            self.items[item_id] = item
            self.item_ids.append(item_id)

    def _build_content_index(self):
        """构建内容索引 (TF-IDF)"""
        if not HAS_SKLEARN or not self.items:
            return

        # 为每个项目构建文本特征
        texts = []
        for item_id in self.item_ids:
            item = self.items[item_id]
            parts = []
            # 名称 (高权重)
            title = item.get("title", "")
            parts.extend([title] * 3)
            # 标签
            tags = item.get("tags", [])
            parts.extend(tags)
            # 类别
            category = item.get("category", "")
            parts.append(category)
            texts.append(" ".join(parts))

        self._tfidf_vectorizer = TfidfVectorizer(
            token_pattern=r"\b\w+\b", stop_words="english", max_features=1000
        )
        self._tfidf_matrix = self._tfidf_vectorizer.fit_transform(texts)

    def _compute_score(self, item: dict, context: dict) -> float:
        """计算单个项目的推荐分数"""
        score = 0.0

        # 1. 内容推荐分
        content_score = self._content_score(item, context)
        score += content_score * 0.4

        # 2. 序列模式分
        sequence_score = self._sequence_score(item, context)
        score += sequence_score * 0.3

        # 3. 热度/频率分
        popularity_score = self._popularity_score(item)
        score += popularity_score * 0.3

        return round(score, 4)

    def _content_score(self, item: dict, context: dict) -> float:
        """内容相似度分"""
        if not HAS_SKLEARN or self._tfidf_matrix is None:
            # 简单回退: 字符串匹配
            return self._simple_content_score(item, context)

        # 构建上下文字符串
        context_parts = []

        # 前台应用标题
        fg_title = context.get("foreground_app_title", "")
        if fg_title:
            context_parts.extend([fg_title] * 2)

        # 浏览器标签
        for tab in context.get("browser_tabs", []):
            context_parts.append(tab)

        # 查询词
        query = context.get("query", "")
        if query:
            context_parts.extend([query] * 3)

        # 前台类别
        fg_cat = context.get("foreground_category", "")
        if fg_cat:
            context_parts.append(fg_cat)

        if not context_parts:
            return 0.0

        context_text = " ".join(context_parts)
        context_vec = self._tfidf_vectorizer.transform([context_text])

        # 找到项目在矩阵中的索引
        item_id = item.get("id", "")
        try:
            idx = self.item_ids.index(item_id)
        except ValueError:
            return 0.0

        item_vec = self._tfidf_matrix[idx : idx + 1]
        sim = cosine_similarity(context_vec, item_vec)[0][0]
        return float(sim)

    def _simple_content_score(self, item: dict, context: dict) -> float:
        """简单的内容匹配 (sklearn 不可用时的回退)"""
        score = 0.0
        title_lower = item.get("title", "").lower()
        tags_lower = [t.lower() for t in item.get("tags", [])]

        # 前台类别匹配
        fg_cat = context.get("foreground_category", "")
        if fg_cat and fg_cat.lower() in tags_lower:
            score += 0.5

        # 名称匹配
        query = context.get("query", "").lower()
        if query and query in title_lower:
            score += 0.3

        return min(score, 1.0)

    def _sequence_score(self, item: dict, context: dict) -> float:
        """序列模式分 (n-gram)"""
        item_id = item.get("id", "")
        recent = context.get("recent_app_ids", [])

        if not recent:
            return 0.0

        # 2-gram: 最近一个应用 → 当前
        last = recent[-1]
        key = (last, item_id)
        count = self._transition_2gram.get(key, 0)

        # 归一化
        total_from_last = sum(
            v for (k, v) in self._transition_2gram.items() if k[0] == last
        )
        if total_from_last == 0:
            return 0.0

        return min(count / total_from_last, 1.0)

    def _popularity_score(self, item: dict) -> float:
        """热度/频率分"""
        item_id = item.get("id", "")
        click_count = self._click_counts.get(item_id, 0)
        launch_count = item.get("launch_count", 0)

        # 合并点击和启动次数
        total = click_count + launch_count

        # 对数归一化到 0-1
        import math

        return min(math.log(total + 1) / 5.0, 1.0)

    # ===== 持久化 =====

    def _save_state(self):
        """保存模型状态到磁盘"""
        state = {
            "click_counts": self._click_counts,
            "transition_2gram": [
                {"from": k[0], "to": k[1], "count": v}
                for k, v in self._transition_2gram.items()
            ],
        }

        state_path = os.path.join(self._data_dir, "state.json")
        try:
            with open(state_path, "w", encoding="utf-8") as f:
                json.dump(state, f, ensure_ascii=False, indent=2)
        except Exception as e:
            logger.warning(f"保存推荐状态失败: {e}")

    def _load_state(self):
        """从磁盘加载模型状态"""
        state_path = os.path.join(self._data_dir, "state.json")
        if not os.path.exists(state_path):
            return

        try:
            with open(state_path, "r", encoding="utf-8") as f:
                state = json.load(f)

            self._click_counts = state.get("click_counts", {})

            transitions = state.get("transition_2gram", [])
            self._transition_2gram = {}
            for t in transitions:
                key = (t["from"], t["to"])
                self._transition_2gram[key] = t["count"]

            logger.info(
                f"Loaded recommend state: {len(self._click_counts)} clicks, "
                f"{len(self._transition_2gram)} transitions"
            )
        except Exception as e:
            logger.warning(f"加载推荐状态失败: {e}")


def create_service() -> Dict[str, callable]:
    """创建推荐服务，返回方法字典供 pybridge 注册"""
    service = RecommendService()
    return {
        "initialize": service.initialize,
        "get_scores": service.get_scores,
        "report_feedback": service.report_feedback,
    }


if __name__ == "__main__":
    # 独立测试
    logging.basicConfig(level=logging.DEBUG)
    service = RecommendService()

    test_items = [
        {"id": "1", "title": "VS Code", "tags": ["dev"], "launch_count": 100},
        {"id": "2", "title": "Chrome", "tags": ["browser"], "launch_count": 80},
        {"id": "3", "title": "Windows Terminal", "tags": ["terminal"], "launch_count": 50},
    ]

    service.initialize({"items": test_items})

    context = {
        "foreground_app_title": "VS Code",
        "foreground_category": "dev",
        "recent_app_ids": ["1"],
    }

    result = service.get_scores({"items": test_items, "context": context})
    print("Scores:")
    for s in result["scores"]:
        print(f"  {s['id']}: {s['score']}")
