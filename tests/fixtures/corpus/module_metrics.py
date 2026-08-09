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
# # tie-aware rank / ERR 지표 module
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %%
"""
평가 지표 모듈

Tie-aware rank 계산 및 ERR(Expected Reciprocal Rank) 지표를 제공합니다.

- calculate_ranks: wide-format score matrix → origin_rank, n_ties, err
- summarize_ranks: ranks DataFrame → top1_acc, mean_err
"""

# %%
import numpy as np

# %%
import pandas as pd

# %%
def calculate_ranks(scores_df: pd.DataFrame, ascending: bool = False, eps: float = 0.0) -> pd.DataFrame:
    """각 query에 대해 origin genome의 tie-aware rank 및 ERR 계산

    Args:
        scores_df: Wide-format DataFrame (index=query_id, columns=ref_id, values=score).
                   Origin = diagonal (query_id == ref_id).
        ascending: True for distance (lower is better), False for similarity (higher is better).
        eps: Tolerance for tie detection. Two scores are tied when |score - origin| <= eps.

    Returns:
        DataFrame with columns: query_id, origin_rank, n_ties, err, total_refs, origin_score
    """
    results = []

    for query_id in scores_df.index:
        if query_id not in scores_df.columns:
            continue

        row = scores_df.loc[query_id]
        origin_score = row[query_id]

        if pd.isna(origin_score):
            continue

        if ascending:
            # Distance: strictly better = score < origin - eps
            strictly_better = (row < origin_score - eps).sum()
            # Ties: |score - origin| <= eps (origin 자신 포함)
            n_ties = ((row - origin_score).abs() <= eps).sum()
        else:
            # Similarity: strictly better = score > origin + eps
            strictly_better = (row > origin_score + eps).sum()
            # Ties: |score - origin| <= eps (origin 자신 포함)
            n_ties = ((row - origin_score).abs() <= eps).sum()

        origin_rank = int(strictly_better) + 1
        n_ties = int(n_ties)

        # ERR = (1/m) * Σ_{i=0}^{m-1} 1/(r+i), where r=origin_rank, m=n_ties
        r = origin_rank
        m = n_ties
        err = sum(1.0 / (r + i) for i in range(m)) / m

        results.append({
            'query_id': query_id,
            'origin_rank': origin_rank,
            'n_ties': n_ties,
            'err': err,
            'total_refs': len(row),
            'origin_score': origin_score,
        })

    return pd.DataFrame(results)

# %%
def summarize_ranks(ranks_df: pd.DataFrame) -> dict:
    """Rank 결과를 요약 통계로 변환

    Args:
        ranks_df: calculate_ranks의 출력 DataFrame

    Returns:
        dict with keys: n_samples, top1_acc, mean_err
    """
    n = len(ranks_df)
    if n == 0:
        return {'n_samples': 0, 'top1_acc': 0.0, 'mean_err': 0.0}

    top1_acc = (ranks_df['origin_rank'] == 1).mean()
    mean_err = ranks_df['err'].mean()

    return {
        'n_samples': n,
        'top1_acc': float(top1_acc),
        'mean_err': float(mean_err),
    }
