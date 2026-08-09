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
# # `.reset_index(drop=True)`를 덧붙인 경우
#
# 마지막에서 두 번째 줄만 고쳤는데 `df_list = []`가 다시 돈다 — 밀려난
# `pd.concat(df_list, axis=0)`이 인자 `df_list`를 in-place로 바꿨을 수 있기
# 때문이다. 세션의 `df_list`에는 이미 두 개가 들어 있으니 빈 리스트를 다시
# 만들지 않으면 값이 어긋난다. 반대로 `RESULT_FILES`는 아무도 건드리지 않았다.

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
merged_df = pd.concat(df_list, axis=0).reset_index(drop=True)

# %%
print(len(merged_df))
