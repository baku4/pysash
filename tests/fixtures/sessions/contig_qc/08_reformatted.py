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
# # 코드는 그대로 두고 모양만 바꾼 경우
#
# 주석 추가, 따옴표 종류, `1000` → `1_000`, 잉여 괄호, 줄바꿈 — 전부 정규화가
# 흡수하는 것들이다. AST가 같으므로 열 개 전부 재사용되어야 한다.
# f-string은 정규화 경계 밖이라 한 글자도 건드리지 않았다.

# %%
from pathlib import Path

# %%
import pandas as pd

# %%
# 데이터 루트
ROOT_DIR = Path("/root/TB")

# %%
PRJ_ROOT_DIR = (ROOT_DIR / 'ref_addition')

# %%
CONTIG_LEN_FILE = (
    PRJ_ROOT_DIR
    / 'contig_len.csv'
)

# %%
MIN_CONTIG_LEN = 1_000  # 1kb 미만은 버린다

# %%
len_df = pd.read_csv(CONTIG_LEN_FILE)


# %%
kept_df = len_df[(len_df['len'] >= MIN_CONTIG_LEN)]

# %%
dropped = (
    len(len_df)
    - len(kept_df)
)

# %%
print(f"dropped {dropped} contigs")
