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
# # 위쪽 임계값을 고친 경우 — 상계의 대가가 보이는 자리
#
# statement 5의 `MIN_CONTIG_LEN`을 1000에서 500으로 내렸다. 편집 지점 아래는 당연히
# 전부 다시 돈다. 흥미로운 것은 편집 지점 **위**다 — 밀려난 `pd.read_csv(...)`는
# 정적으로는 receiver `pd`와 인자 `CONTIG_LEN_FILE`을 in-place로 바꿨을 수 있는
# 실행이라(순수 화이트리스트 밖 메서드 호출의 상계), 그 둘을 만든 statement 1과 4가
# 함께 다시 돈다.
# `from pathlib import Path`와 경로 상수 두 개는 아무도 건드리지 않아 재사용된다.

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
MIN_CONTIG_LEN = 500

# %%
len_df = pd.read_csv(CONTIG_LEN_FILE)

# %%
kept_df = len_df[len_df['len'] >= MIN_CONTIG_LEN]

# %%
dropped = len(len_df) - len(kept_df)

# %%
print(f"dropped {dropped} contigs")
