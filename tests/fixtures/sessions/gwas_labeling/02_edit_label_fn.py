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
# # 라벨링 함수 본문만 고친 경우
#
# `def label_locus`(index 4)에 분기를 하나 더했다. `import pandas as pd`와 파일
# 경로 상수, 그리고 `DRUG_LIST`는 그대로 재사용된다 — 편집 지점 아래의 어떤
# 실행도 그 이름들을 건드리지 않기 때문이다.
#
# 다만 `gwas_df = pd.read_csv(...)`는 다시 돈다. 밀려난
# `gwas_df['locus_tag'].apply(...)`가 receiver `gwas_df`를 in-place로 바꿨을 수
# 있다는 상계 때문이다. Run이 연속 구간이 아니라는 점이 이 케이스의 핵심이다.

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
    if locus_tag is None:
        return 'unknown'
    return 'intergenic'

# %%
labeled = gwas_df['locus_tag'].apply(label_locus)

# %%
print(labeled.value_counts())
