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
# # 외부 툴 실행 module
#
# pysash의 정렬 fixture다. 실행하지 않는다 — 파싱과 판정에만 쓴다.

# %%
"""
툴 실행 모듈

Conda 환경에서 외부 툴을 실행하는 기능을 제공합니다.
"""

# %%
import subprocess

# %%
import time

# %%
from pathlib import Path

# %%
from typing import List, Optional, Tuple

# %%
from .config import get_project_root, load_config

# %%
def run_tool_in_env(env_name: str, command: List[str], timeout: Optional[int] = None) -> Tuple[int, str, str]:
    """
    Conda 환경에서 툴 실행

    Args:
        env_name: conda 환경 이름
        command: 실행할 명령어 리스트
        timeout: 타임아웃 (초)

    Returns:
        (return_code, stdout, stderr) 튜플
    """
    config = load_config()
    activate_script = config['conda']['activate_script']

    full_cmd = f"source {activate_script} && mamba activate {env_name} && {' '.join(command)}"

    start_time = time.time()
    try:
        result = subprocess.run(
            full_cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout,
            executable='/bin/bash'
        )
        elapsed = time.time() - start_time
        print(f"  [실행 시간: {elapsed:.2f}s]")
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", f"Timeout after {timeout}s"

# %%
def run_kmer_meter(fasta_or_fastq: Path, index: Path, output: Path,
                   mode: str = "containment", **kwargs) -> Path:
    """
    kmer-meter 실행

    Args:
        fasta_or_fastq: query 파일
        index: 인덱스 파일
        output: 출력 파일
        mode: 실행 모드 (containment, occ-weighted, qual-weighted, occ-qual-weighted)
        **kwargs: 추가 옵션

    Returns:
        출력 파일 경로
    """
    config = load_config()
    kmer_meter_bin = get_project_root() / config['tools']['kmer_meter']['path']

    cmd = [
        str(kmer_meter_bin),
        "query",
        "--index", str(index),
        "--query", str(fasta_or_fastq),
        "--output", str(output),
        "--mode", mode
    ]

    # 추가 옵션
    for key, value in kwargs.items():
        cmd.extend([f"--{key.replace('_', '-')}", str(value)])

    returncode, stdout, stderr = run_tool_in_env("kmeter", cmd)

    if returncode != 0:
        raise RuntimeError(f"kmer-meter failed: {stderr}")

    return output
