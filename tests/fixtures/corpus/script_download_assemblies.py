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
# # E. coli assembly 내려받기
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %% [markdown]
# # NCBI E. coli Assembly 다운로드
#
# NCBI RefSeq에서 E. coli assembly를 다운로드합니다.
#
# **입력**:
# - `metadata/ncbi_dataset.tsv` - NCBI E. coli assembly 메타데이터 (47,708개)
#
# **출력**:
# - `data/02-EcoliOrigin/01_download/genomes/{accession}.fna` - FASTA 파일
#
# **도구**:
# - ncbi-datasets-cli (`datasets download genome accession`)

# %%
import pandas as pd
from pathlib import Path
import sys
import subprocess

sys.path.append('..')
from src import runner
from src.config import load_config, get_path

# %%
config = load_config()
genomes_dir = get_path('ecoli_genomes')
genomes_dir.mkdir(parents=True, exist_ok=True)

tmp_dir = get_path('tmp')
tmp_dir.mkdir(parents=True, exist_ok=True)

print(f"Genomes directory: {genomes_dir}")
print(f"Temp directory: {tmp_dir}")

# %%
from src.config import get_project_root
metadata_path = get_project_root() / "metadata" / "ncbi_dataset.tsv"
print(f"\nLoading metadata from: {metadata_path}")

metadata = pd.read_csv(metadata_path, sep='\t')
print(f"Total assemblies: {len(metadata):,}")

# Assembly Level 분포
print("\nAssembly Level distribution:")
print(metadata['Assembly Level'].value_counts())

# %%
accessions = metadata['Assembly Accession'].tolist()
print(f"\nTotal accessions: {len(accessions):,}")

# 대표 accession 확인
rep_accession = config['test1_ecoli_origin']['representative_accession']
if rep_accession in accessions:
    print(f"Representative accession found: {rep_accession}")
else:
    print(f"WARNING: Representative accession {rep_accession} not found in metadata!")

# %%
existing = set(f.stem for f in genomes_dir.glob("*.fna"))
to_download = [acc for acc in accessions if acc not in existing]

print(f"\nAlready downloaded: {len(existing):,}")
print(f"To download: {len(to_download):,}")

# %% [markdown]
# ---
# ## 다운로드 작업 제출

# %%
def download_genome(accession: str, output_dir: Path, tmp_dir: Path) -> str:
    """
    단일 게놈 다운로드 명령어 생성

    1. datasets download genome accession {accession} --include genome
    2. unzip ncbi_dataset.zip
    3. mv fna file to output_dir/{accession}.fna
    4. cleanup
    """
    work_dir = tmp_dir / accession
    zip_file = work_dir / "ncbi_dataset.zip"
    output_file = output_dir / f"{accession}.fna"

    cmd = f'''
mkdir -p "{work_dir}"
cd "{work_dir}"
/root/miniforge3/bin/mamba run -n ncbi-datasets datasets download genome accession {accession} --include genome --filename ncbi_dataset.zip
if [ -f ncbi_dataset.zip ]; then
    unzip -o ncbi_dataset.zip
    # FASTA 파일 찾기 및 이동
    fna_file=$(find ncbi_dataset/data -name "*.fna" | head -1)
    if [ -n "$fna_file" ]; then
        mv "$fna_file" "{output_file}"
        echo "OK: {accession}"
    else
        echo "ERROR: No FNA file found for {accession}"
    fi
    # 정리
    rm -rf ncbi_dataset ncbi_dataset.zip
else
    echo "ERROR: Download failed for {accession}"
fi
cd /
rm -rf "{work_dir}"
'''
    return cmd

# %%
print("\n" + "="*60)
print("다운로드 준비")
print("="*60)

PARALLEL = 8

if to_download:
    commands = [download_genome(acc, genomes_dir, tmp_dir) for acc in to_download]
    runner.prepare(commands, group="ncbi_download")
    runner.run("ncbi_download", parallel=PARALLEL)
else:
    print("Nothing to download.")

# %% [markdown]
# ---
# ## 진행 상황 모니터링

# %%
def check_progress():
    """다운로드 진행 상황 확인"""
    downloaded = len(list(genomes_dir.glob("*.fna")))

    print("="*60)
    print("Download Progress")
    print("="*60)
    print(f"Downloaded: {downloaded:,} / {len(accessions):,} ({downloaded/len(accessions)*100:.1f}%)")
    runner.status("ncbi_download")

# 실행: check_progress()

# %%
check_progress()

# %% [markdown]
# ---
# ## 다음 단계
#
# 다운로드가 완료되면 `02_02_run_skani.py`를 실행하여
# 대표 게놈과의 ANI를 계산합니다.
