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
# # GWAS 결과 라벨링 — 아래쪽에 정의한 헬퍼 함수
#
# `corpus/notebook_gwas_manhattan.py`의 `labeling_gwas_res` 패턴.
# 무거운 `read_csv`를 먼저 하고, 라벨링 함수는 아래에서 고쳐 가며 반복한다 —
# 셸에서 실제로 가장 자주 하는 편집이다.

# %%
import pandas as pd

# %%
GWAS_RESULT_FILE = 'gwas_annotated.csv'

# %%
gwas_df = pd.read_csv(GWAS_RESULT_FILE)

# %%
DRUG_LIST = ['RIF', 'INH', 'EMB']

# %%
def label_locus(locus_tag):
    if isinstance(locus_tag, str):
        return 'on_transcript'
    return 'intergenic'

# %%
labeled = gwas_df['locus_tag'].apply(label_locus)

# %%
print(labeled.value_counts())
