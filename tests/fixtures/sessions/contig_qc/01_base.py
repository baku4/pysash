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
# # contig 길이 QC — 처음 실행한 그대로
#
# `corpus/notebook_contig_length_qc.py`를 self-contained하게 줄인 것.
# 셀 하나에 top-level statement 하나씩 두어 index와 셀이 1:1로 맞는다.
#
# statement 0..9 — 판정을 손으로 따라갈 수 있도록 순서가 곧 index다.

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
