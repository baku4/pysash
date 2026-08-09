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
# # 아래로 이어 붙이기 — 셸에서 다음 셀을 치는 경우
#
# `01_base.py` 뒤에 statement 두 개(10, 11)를 덧붙였다. 앞의 열 개는 글자 하나
# 바뀌지 않았고, 세션이 그 소스의 순수 prefix이므로 residue가 없다.

# %%
from pathlib import Path

# %%
import pandas as pd

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

# %%
kept_df.to_csv(PRJ_ROOT_DIR / 'kept_contigs.csv', index=False)

# %%
print(f"saved {len(kept_df)} contigs")
