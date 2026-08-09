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
# # ENA 업로드용 assembly manifest 만들기
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # Describe the assembly

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
# ## Parse Configs

# %%
root_dir = Path(config['root_dir'])
prj_root_dir = root_dir / 'upload_ena'
prj_root_dir.mkdir(exist_ok=True)
prj_root_dir

# %% [markdown]
# ### Input

# %%
assem_root_dir = root_dir / 'assembly'
mtbrf_fa = assem_root_dir / 'mtbrf.fa'
mtbrf_gff = assem_root_dir / 'mtbrf.gff'
mtbrf_fa, mtbrf_gff

# %% [markdown]
# ### Output

# %%
manifest_file = prj_root_dir / 'menifest.txt'
manifest_file

# %% [markdown]
# ## 1. Encode to EMBL file

# %%
out_embl_file = prj_root_dir / 'result.embl.gz'
out_embl_file

# %%
conda_env = "emblmygff3"

locus_tag = "SNGC"
species = 'Mycobacterium tuberculosis'
project_acc = 'PRJEB66375'

# %%
cmd = f"EMBLmyGFF3 {mtbrf_gff} {mtbrf_fa} \
    --molecule_type 'genomic DNA' \
    --strain 'MtbRf' \
    --transl_table 11  \
    --topology circular \
    --species 1773 \
    --locus_tag {locus_tag} \
    --project_id {project_acc} \
    -z \
    -o {out_embl_file}"
cmd

# %% [markdown]
# > Run on env manually

# %% [markdown]
# ### Delete errored fetures

# %%
filtred_out_embl_file = out_embl_file.with_stem('filtered_embl')
filtred_out_embl_file

# %%
to_del_list = [
    "/anticodon=",
    "ribosomal slippage",
]

# %%
import gzip
with gzip.open(out_embl_file, 'rt') as in_hdl:
    with gzip.open(filtred_out_embl_file, 'wt') as out_hdl:
        for l in in_hdl:
            write = True
            for s in to_del_list:
                if s in l:
                    write = False
                    break
            if write:
                out_hdl.write(l)

# %% [markdown]
# ## 2. Chromosome List File

# %%
chr_list_file = prj_root_dir / 'chrom_list.txt'
chr_list_file

# %%
with open(chr_list_file, 'w') as f:
    f.write("mtbrf\t1\tcircular-chromosome")

# %%
gzip_cmd = f"gzip {chr_list_file}"
subprocs(gzip_cmd)

# %% [markdown]
# ## 3. Manifest file

# %%
data = [
    ("STUDY", study_acc),
    ("SAMPLE", "ERS16431646"),
    ("ASSEMBLYNAME", "MtbRf"),
    ("ASSEMBLY_TYPE", "isolate"),
    ("COVERAGE", "100"),
    ("PROGRAM", "MEGAHIT"),
    ("PLATFORM", "Illumina MiSeq"),
    ("MOLECULETYPE", "genomic DNA"),
    ("FLATFILE", filtred_out_embl_file),
    ("CHROMOSOME_LIST", str(chr_list_file.with_suffix(f"{chr_list_file.suffix}.gz"))),
]

manifest_df = pd.DataFrame(data, columns=['name', 'value'])
manifest_df

# %%
manifest_df.to_csv(manifest_file, sep='\t', index=False, header=None)

# %%
manifest_file
