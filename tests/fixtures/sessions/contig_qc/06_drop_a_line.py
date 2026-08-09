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
# # 아래쪽 셀 하나를 지운 경우
#
# `dropped = ...`(index 8)를 지우고 마지막 출력을 그 자리로 당겼다. 밀려나는 실행은
# 지워진 대입과 옛 `print` 둘뿐이고, 둘 다 위쪽이 만든 것을 건드리지 않는다.
# 그래서 앞의 여덟 개가 그대로 재사용된다.

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
print(f"kept {len(kept_df)} contigs")
