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
# # 맨 아래 한 줄만 고친 경우 — 이 도구의 헤드라인
#
# statement 9의 출력 문구만 바꿨다. 밀려나는 실행은 그 `print` 하나뿐이고, print는
# 아무 이름도 바인딩하지 않고 인자를 in-place로 바꾸지도 않는다(순수 호출
# 화이트리스트). 따라서 위의 아홉 개는 전부 그대로 재사용되어야 한다.

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
print(f"dropped {dropped} of {len(len_df)} contigs")
