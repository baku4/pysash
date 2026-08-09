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
# # 중간에 셀 하나를 끼워 넣은 경우
#
# `pd.set_option(...)`을 index 6 자리에 새로 넣었다. 삽입 지점 아래는 위치가 통째로
# 한 칸씩 밀리므로 canonical이 같아도 "그 자리의 실행"이 아니다 — 전부 Run이다.
# 위쪽은 `04_edit_threshold.py`와 같은 이유로 1과 4만 다시 돈다.

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
pd.set_option('display.max_rows', 200)

# %%
len_df = pd.read_csv(CONTIG_LEN_FILE)

# %%
kept_df = len_df[len_df['len'] >= MIN_CONTIG_LEN]

# %%
dropped = len(len_df) - len(kept_df)

# %%
print(f"dropped {dropped} contigs")
