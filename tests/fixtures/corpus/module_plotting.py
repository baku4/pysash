# ---
# jupyter:
#   jupytext:
#     text_representation:
#       extension: .py
#       format_name: percent
#       format_version: '1.3'
#   kernelspec:
#     display_name: Python 3
#     language: python
#     name: python3
# ---


# %% [markdown]
# # 벤치마크 시각화 module
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %%
"""
시각화 모듈

벤치마크 결과를 시각화하는 함수들을 제공합니다.
"""

# %%
from pathlib import Path

# %%
from typing import Dict, Optional

# %%
import matplotlib.pyplot as plt

# %%
import pandas as pd

# %%
import seaborn as sns

# %%
def plot_rank_distribution(predictions: pd.DataFrame, ground_truth: pd.DataFrame,
                           title: str = "Rank Distribution", output: Optional[Path] = None):
    """
    True genome의 rank 분포 시각화

    Args:
        predictions: 예측 결과
        ground_truth: 정답
        title: 그래프 제목
        output: 저장 경로 (None이면 표시만)
    """
    merged = predictions.merge(ground_truth, on='query_id')
    correct_ranks = merged[merged['genome_id'] == merged['true_genome_id']]['rank']

    plt.figure(figsize=(10, 6))
    plt.hist(correct_ranks, bins=range(1, correct_ranks.max() + 2), edgecolor='black', alpha=0.7)
    plt.xlabel('Rank of True Genome')
    plt.ylabel('Count')
    plt.title(title)
    plt.grid(axis='y', alpha=0.3)

    if output:
        plt.savefig(output, dpi=300, bbox_inches='tight')
    else:
        plt.show()

    plt.close()

# %%
def plot_tool_comparison(results: Dict[str, Dict[str, float]],
                         metric_name: str = "Top-1 Accuracy",
                         output: Optional[Path] = None):
    """
    여러 툴의 성능 비교 막대 그래프

    Args:
        results: {tool_name: {metric: value}} 형태의 딕셔너리
        metric_name: 표시할 지표 이름
        output: 저장 경로
    """
    tools = list(results.keys())
    values = [results[tool][metric_name] for tool in tools]

    plt.figure(figsize=(12, 6))
    bars = plt.bar(tools, values, edgecolor='black', alpha=0.7)

    # 값 표시
    for bar in bars:
        height = bar.get_height()
        plt.text(bar.get_x() + bar.get_width()/2., height,
                f'{height:.3f}', ha='center', va='bottom')

    plt.xlabel('Tool')
    plt.ylabel(metric_name)
    plt.title(f'{metric_name} Comparison Across Tools')
    plt.xticks(rotation=45, ha='right')
    plt.grid(axis='y', alpha=0.3)
    plt.tight_layout()

    if output:
        plt.savefig(output, dpi=300, bbox_inches='tight')
    else:
        plt.show()

    plt.close()
