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
# # 결과 파일 모아 붙이기 — 누산 리스트
#
# `corpus/notebook_compare_tools.py`의 `df_list = []` → 루프에서
# `append` → `pd.concat` 패턴. 실제 notebook에서 가장 흔한 모양이고, in-place
# 누산이 재사용 판정에 어떻게 걸리는지 그대로 보여준다.

# %%
import pandas as pd

# %%
RESULT_FILES = ['sigalign.csv', 'other_tools.csv']

# %%
df_list = []

# %%
for f in RESULT_FILES:
    df_list.append(pd.read_csv(f))

# %%
merged_df = pd.concat(df_list, axis=0)

# %%
print(len(merged_df))
