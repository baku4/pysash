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
# # DST 예측용 variant 표 만들기
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Variants Table

# %%
import os, sys
from pathlib import Path
import json
import subprocess

import pandas as pd
import numpy as np

# %%
# import utils.py
currentdir = os.path.dirname(os.path.realpath(__name__))
parentdir = os.path.dirname(currentdir)
sys.path.append(parentdir)

from utils import *

# %%
config_file = "../config.toml"
config = parse_config(config_file)

# %% [markdown]
# ### input

# %%
root_dir = Path(config['root_dir'])
prj_root_dir = root_dir / 'dst_pred_6'
prj_root_dir.mkdir(exist_ok=True)
prj_root_dir

# %%
reseq_addt_root_dir = root_dir / 'reseq_mtbrf'
aln_fpath_file = reseq_addt_root_dir / 'aln_fpath.csv'
aln_fpath_file

# %%
assem_root_dir = root_dir / 'assembly'
ref_fa_file = assem_root_dir / 'mtbrf.fa'
ref_gff_file = assem_root_dir / 'mtbrf.gff'
ref_fa_file, ref_gff_file

# %%
out_vcf_file = reseq_addt_root_dir / 'out.vcf'
out_vcf_file

# %%
reseq_h37rv_root_dir = root_dir / 'reseq_h37rv'
h37rv_vcf_file = reseq_h37rv_root_dir / 'out.vcf'
h37rv_vcf_file

# %% [markdown]
# ### output

# %%
h37rv_var_desc_file = prj_root_dir / 'h37rv_var_desc.csv'
addt_var_desc_file = prj_root_dir / 'addt_var_desc.csv'
h37rv_var_desc_file, addt_var_desc_file

# %%
h37rv_var_table_file = prj_root_dir / 'h37rv_var_table.csv'
addt_var_table_file = prj_root_dir / 'addt_var_table.csv'
h37rv_var_table_file, addt_var_table_file

# %% [markdown]
# ## 1. Variants description

# %%
import vcfpy

# %%
def get_var_desc_df(vcf_file):
    desc_list = []
    
    with vcfpy.Reader.from_path(vcf_file) as vcf_reader:
        for idx, record in enumerate(vcf_reader):
            chrom = record.CHROM
            pos = record.POS
            is_snv = record.is_snv()
            genotypes = [record.REF] + [alt.value for alt in record.ALT]
            genotypes = '|'.join(genotypes)

            ser1 = pd.Series(
                [chrom, pos, is_snv, genotypes],
                index = ['chrom', 'pos', 'is_snv', 'genotypes'],
            )

            calls = [
                call.gt_alleles[0] if call.called else -1
                for call
                in record.calls
            ]
            ser2 = pd.Series(calls).value_counts()

            desc_list.append(pd.concat([ser1, ser2]))
            
            if idx % 10_000 == 0:
                print(f"current idx: {idx}")
    
    var_desc_df = pd.concat(desc_list, axis=1).transpose()
    cols = var_desc_df.columns
    count_cols = list(cols[4:])
    count_cols.sort()
    new_cols = list(cols[:4]) + count_cols
    var_desc_df = var_desc_df[new_cols]
    var_desc_df = var_desc_df.fillna(0)
    
    return var_desc_df

# %%
# h37rv_var_desc_df = get_var_desc_df(h37rv_vcf_file)

# %% [markdown]
# ---

# %%
# h37rv_var_desc_df.to_csv(h37rv_var_desc_file, index=False)

# %%
# h37rv_var_desc_df = pd.read_csv(h37rv_var_desc_file)
# h37rv_var_desc_df

# %%
# addt_var_desc_df = get_var_desc_df(out_vcf_file)

# %%
# addt_var_desc_df.to_csv(addt_var_desc_file, index=False)

# %%
addt_var_desc_df = pd.read_csv(addt_var_desc_file)
addt_var_desc_df

# %%
addt_var_desc_df['chrom'].value_counts()

# %%
# sum(addt_var_desc_df['chrom'] != 'NC_000962.3')

# %%
# 4_411_532 / 40_739, 269_634 / 2_009

# %% [markdown]
# ## 2. Varaint table

# %%
aln_fpath_df = pd.read_csv(aln_fpath_file)
aln_fpath_df.head()

# %%
bam_to_id_dict = dict(zip(
    aln_fpath_df['dupmarked_bam'].apply(lambda x: Path(x).stem),
    aln_fpath_df['running_id'].values
))

# %%
def get_var_table_df(
    vcf_file,
    bam_to_id_dict=bam_to_id_dict,
):
    
    data = []
    
    with vcfpy.Reader.from_path(vcf_file) as vcf_reader:
        samples = [
            bam_to_id_dict[Path(bam).stem]
            for bam
            in vcf_reader.header.samples.names
        ]
        
        for idx, record in enumerate(vcf_reader):
            data.append([
                call.gt_alleles[0] if call.called else -1
                for call
                in record.calls
            ])
            if idx % 10_000 == 0:
                print(f"idx: {idx}")

    calls_df = pd.DataFrame(data, columns=samples)
    
    return calls_df

# %%
# addt_var_table_df = get_var_table_df(out_vcf_file)

# %%
# addt_var_table_df.to_csv(addt_var_table_file, index=False)

# %%
addt_var_table_df = pd.read_csv(addt_var_table_file)

# %%
addt_var_table_df

# %%
# h37rv_var_table_df = get_var_table_df(h37rv_vcf_file)

# %%
# h37rv_var_table_df.to_csv(h37rv_var_table_file, index=False)

# %%
# h37rv_var_table_df = pd.read_csv(h37rv_var_table_file)

# %%
# h37rv_var_table_df

# %% [markdown]
# ---
# # Test
