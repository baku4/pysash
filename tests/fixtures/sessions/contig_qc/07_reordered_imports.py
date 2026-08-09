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
# # 맨 위 두 줄의 순서만 바꾼 경우
#
# 내용은 하나도 바뀌지 않았지만 index 0의 statement가 달라졌다. 세션은 linear한
# 실행 기록이고 prefix는 위치로 고정되므로 공통 prefix가 0이 된다 — 전부 다시 돈다.
# jupyter처럼 위아래를 오가는 모델을 이 도구가 표현하지 않는다는 뜻이다.

# %%
import pandas as pd

# %%
from pathlib import Path

# %%
ROOT_DIR = Path('/root/TB')

# %%
PRJ_ROOT_DIR = ROOT_DIR / 'ref_addition'

# %%
CONTIG_LEN_FILE = PRJ_ROOT_DIR / 'contig_len.csv'

# %%
MIN_CONTIG_LEN = 1000

# %%
len_df = pd.read_csv(CONTIG_LEN_FILE)

# %%
kept_df = len_df[len_df['len'] >= MIN_CONTIG_LEN]

# %%
dropped = len(len_df) - len(kept_df)

# %%
print(f"dropped {dropped} contigs")
