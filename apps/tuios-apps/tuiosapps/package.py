"""Ciclo de pacote: empacotar / adicionar / remover (tar.gz + SHA256SUMS, R9–R11)."""
from __future__ import annotations

import hashlib
import io
import re
import shutil
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

from .manifest import ErroManifesto, carregar_manifesto

NOME_SUMS = "SHA256SUMS"
EXCLUIR_DIRS = {".git", "__pycache__"}
EXCLUIR_SUFIXOS = (".pyc",)
LINHA_SUMS_RE = re.compile(r"^([0-9a-f]{64})  (.+)$")


class ErroPacote(Exception):
    """Pacote inválido/instalação recusada — o CLI converte em exit 2."""


def _incluir(rel: str) -> bool:
    partes = PurePosixPath(rel).parts
    if any(p in EXCLUIR_DIRS for p in partes):
        return False
    if rel.endswith(EXCLUIR_SUFIXOS):
        return False
    if partes and partes[-1] == NOME_SUMS:
        return False
    return True


def empacotar(dir_app: Path, saida: Path | None = None) -> Path:
    """Valida o manifesto e gera <saida>.tar.gz prefixado com o nome do app."""
    m = carregar_manifesto(dir_app)
    destino = saida if saida is not None else Path(f"{m.nome}-{m.versao}.tar.gz")

    arquivos = [
        p
        for p in sorted(dir_app.rglob("*"))
        if p.is_file() and _incluir(p.relative_to(dir_app).as_posix())
    ]
    linhas = [
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  "
        f"{p.relative_to(dir_app).as_posix()}"
        for p in arquivos
    ]
    sums = ("\n".join(linhas) + "\n").encode("utf-8")

    with tarfile.open(destino, "w:gz") as tar:
        for p in arquivos:
            tar.add(p, arcname=f"{m.nome}/{p.relative_to(dir_app).as_posix()}")
        ti = tarfile.TarInfo(name=f"{m.nome}/{NOME_SUMS}")
        ti.size = len(sums)
        ti.mtime = int(dir_app.stat().st_mtime)
        tar.addfile(ti, io.BytesIO(sums))
    return destino


def verificar_checksum(dir_app: Path) -> None:
    """Confere SHA256SUMS contra o conteúdo real (listagem exata, dois sentidos)."""
    arquivo = dir_app / NOME_SUMS
    if not arquivo.is_file():
        raise ErroPacote(f"{NOME_SUMS} ausente no pacote")
    esperados: dict[str, str] = {}
    for linha in arquivo.read_text(encoding="utf-8").splitlines():
        if not linha:
            continue
        m = LINHA_SUMS_RE.match(linha)
        if not m:
            raise ErroPacote(f"{NOME_SUMS} malformado: {linha!r}")
        esperados[m.group(2)] = m.group(1)

    atuais: dict[str, str] = {}
    for p in sorted(dir_app.rglob("*")):
        if not p.is_file():
            continue
        rel = p.relative_to(dir_app).as_posix()
        if rel == NOME_SUMS:
            continue
        atuais[rel] = hashlib.sha256(p.read_bytes()).hexdigest()

    if set(esperados) != set(atuais):
        faltando = sorted(set(esperados) - set(atuais))
        extra = sorted(set(atuais) - set(esperados))
        raise ErroPacote(
            f"{NOME_SUMS} divergente (faltando: {faltando}, extra: {extra})"
        )
    for rel, digest in esperados.items():
        if atuais[rel] != digest:
            raise ErroPacote(f"checksum divergente: {rel}")


def adicionar(tarball: Path, destino: Path) -> Path:
    """Valida o pacote em staging e só então instala em <destino>/<nome>."""
    if not tarball.is_file():
        raise ErroPacote(f"pacote nao encontrado: {tarball}")
    try:
        tar = tarfile.open(tarball, "r:gz")
    except (tarfile.ReadError, OSError) as exc:
        raise ErroPacote(f"pacote invalido (gzip/tar): {exc}") from exc

    with tar:
        membros = tar.getmembers()
        if not membros:
            raise ErroPacote("pacote vazio")
        prefixos = {
            PurePosixPath(m.name).parts[0]
            for m in membros
            if PurePosixPath(m.name).parts
        }
        if len(prefixos) != 1:
            raise ErroPacote(
                f"pacote deve ter um unico diretorio raiz; encontrado: {sorted(prefixos)}"
            )
        raiz = prefixos.pop()
        if raiz in (".", ".."):
            raise ErroPacote(f"raiz insegura no pacote: {raiz!r}")
        for m in membros:
            pp = PurePosixPath(m.name)
            if pp.is_absolute() or ".." in pp.parts:
                raise ErroPacote(f"entrada insegura: {m.name}")
            if m.issym() or m.islnk():
                alvo_link = PurePosixPath(m.linkname)
                if alvo_link.is_absolute() or ".." in alvo_link.parts:
                    raise ErroPacote(f"link inseguro: {m.name}")

        with tempfile.TemporaryDirectory(prefix="tuios-apps-") as tmp:
            try:
                tar.extractall(tmp, filter="data")
            except (tarfile.FilterError, ValueError) as exc:
                raise ErroPacote(f"extracao recusada: {exc}") from exc
            candidato = Path(tmp) / raiz
            try:
                m = carregar_manifesto(candidato)
            except ErroManifesto:
                raise
            if m.nome != raiz:
                raise ErroPacote(
                    f"nome do manifesto ({m.nome}) difere da raiz do pacote ({raiz})"
                )
            verificar_checksum(candidato)

            destino.mkdir(parents=True, exist_ok=True)
            alvo_final = destino / raiz
            if alvo_final.exists():
                raise ErroPacote(f"app ja instalado: {m.nome}")
            shutil.move(str(candidato), str(alvo_final))
    return alvo_final
