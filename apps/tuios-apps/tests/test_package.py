import io
import shutil
import tarfile
from pathlib import Path

import pytest

from tuiosapps.manifest import ErroManifesto
from tuiosapps.package import ErroPacote, adicionar, empacotar, verificar_checksum


def test_empacotar_gera_targz_com_prefixo_e_sums(app_valido: Path, tmp_path: Path):
    saida = tmp_path / "app.tar.gz"
    resultado = empacotar(app_valido, saida)
    assert resultado == saida and saida.is_file()
    with tarfile.open(saida, "r:gz") as tar:
        nomes = tar.getnames()
    assert "ola-tuios/tuios-app.toml" in nomes
    assert "ola-tuios/ola.prw" in nomes
    assert "ola-tuios/SHA256SUMS" in nomes


def test_empacotar_exclui_lixo(app_valido: Path, tmp_path: Path):
    (app_valido / ".git").mkdir()
    (app_valido / ".git" / "config").write_text("x", encoding="utf-8")
    (app_valido / "__pycache__").mkdir()
    (app_valido / "__pycache__" / "a.pyc").write_bytes(b"x")
    (app_valido / "velho.pyc").write_bytes(b"x")
    saida = empacotar(app_valido, tmp_path / "app.tar.gz")
    with tarfile.open(saida, "r:gz") as tar:
        nomes = tar.getnames()
    assert not any(".git" in n or "__pycache__" in n or n.endswith(".pyc") for n in nomes)


def test_empacotar_manifesto_invalido_falha(app_valido: Path, tmp_path: Path):
    (app_valido / "tuios-app.toml").write_text("nome = [x", encoding="utf-8")
    with pytest.raises(ErroManifesto):
        empacotar(app_valido, tmp_path / "app.tar.gz")


def test_roundtrip_empacotar_adicionar(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    alvo = adicionar(tarball, destino)
    assert alvo == destino / "ola-tuios"
    assert (alvo / "tuios-app.toml").is_file()
    assert (alvo / "ola.prw").is_file()
    assert (alvo / "SHA256SUMS").is_file()


def test_adicionar_nao_gzip(tmp_path: Path):
    lixo = tmp_path / "lixo.tar.gz"
    lixo.write_bytes(b"nao sou gzip")
    with pytest.raises(ErroPacote, match="pacote invalido"):
        adicionar(lixo, tmp_path / "destino")


def test_adicionar_prefixos_multiplos_rejeitado(tmp_path: Path):
    # pacote com DOIS diretórios raiz → rejeita
    a = tmp_path / "origem"
    (a / "um").mkdir(parents=True)
    (a / "dois").mkdir(parents=True)
    (a / "um" / "t.txt").write_text("x", encoding="utf-8")
    (a / "dois" / "t.txt").write_text("x", encoding="utf-8")
    tarball = tmp_path / "dup.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(a / "um", arcname="um")
        tar.add(a / "dois", arcname="dois")
    with pytest.raises(ErroPacote, match="unico diretorio"):
        adicionar(tarball, tmp_path / "destino")


def _tar_malicioso(tmp_path: Path, nome_entrada: str) -> Path:
    tarball = tmp_path / "malicioso.tar.gz"
    dados = b"pwn"
    with tarfile.open(tarball, "w:gz") as tar:
        ti = tarfile.TarInfo(name=nome_entrada)
        ti.size = len(dados)
        tar.addfile(ti, io.BytesIO(dados))
    return tarball


def test_adicionar_rejeita_path_traversal(tmp_path: Path):
    tarball = _tar_malicioso(tmp_path, "app/../escape.txt")
    with pytest.raises(ErroPacote, match="insegura"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_rejeita_absoluto(tmp_path: Path):
    tarball = _tar_malicioso(tmp_path, "/etc/passwd")
    with pytest.raises(ErroPacote, match="insegura"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_checksum_divergente(app_valido: Path, tmp_path: Path):
    # monta pacote e depois troca o conteúdo do entry sem atualizar SHA256SUMS
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    refeito = tmp_path / "refeito.tar.gz"
    with tarfile.open(tarball, "r:gz") as origem, tarfile.open(refeito, "w:gz") as novo:
        for m in origem.getmembers():
            dados = origem.extractfile(m).read() if m.isreg() else None
            if m.name == "ola-tuios/ola.prw":
                dados = b"Return .F.  // adulterado"
                m.size = len(dados)
            novo.addfile(m, io.BytesIO(dados) if dados is not None else None)
    with pytest.raises(ErroPacote, match="checksum"):
        adicionar(refeito, tmp_path / "destino")


def test_adicionar_sem_sums_rejeitado(app_valido: Path, tmp_path: Path):
    tarball = tmp_path / "sem-sums.tar.gz"
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(app_valido, arcname="ola-tuios")
    with pytest.raises(ErroPacote, match="SHA256SUMS"):
        adicionar(tarball, tmp_path / "destino")


def test_adicionar_app_ja_instalado(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    adicionar(tarball, destino)
    with pytest.raises(ErroPacote, match="ja instalado"):
        adicionar(tarball, destino)


def test_adicionar_manifesto_invalido_no_pacote(app_valido: Path, tmp_path: Path):
    # pacote válido é gerado; o manifesto é adulterado DENTRO do tarball
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    refeito = tmp_path / "refeito.tar.gz"
    invalido = 'nome = "ola-tuios"\nversao = "x"\ndescricao = "d"\nentry = "ola.prw"\n'
    with tarfile.open(tarball, "r:gz") as origem, tarfile.open(refeito, "w:gz") as novo:
        for m in origem.getmembers():
            dados = origem.extractfile(m).read() if m.isreg() else None
            if m.name == "ola-tuios/tuios-app.toml":
                dados = invalido.encode("utf-8")
                m.size = len(dados)
            novo.addfile(m, io.BytesIO(dados) if dados is not None else None)
    # manifesto é validado ANTES do checksum → ErroManifesto (exit 2 no CLI)
    with pytest.raises(ErroManifesto):
        adicionar(refeito, tmp_path / "destino")


def test_verificar_checksum_ok(app_valido: Path, tmp_path: Path):
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    destino = tmp_path / "usuario"
    destino.mkdir()
    alvo = adicionar(tarball, destino)
    verificar_checksum(alvo)  # não levanta


def test_verificar_checksum_faltando_arquivo(app_valido: Path, tmp_path: Path):
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    destino = tmp_path / "usuario"
    destino.mkdir()
    alvo = adicionar(tarball, destino)
    (alvo / "ola.prw").unlink()
    with pytest.raises(ErroPacote, match="divergente"):
        verificar_checksum(alvo)


def test_remover_rmtree(app_valido: Path, tmp_path: Path):
    destino = tmp_path / "usuario"
    destino.mkdir()
    tarball = empacotar(app_valido, tmp_path / "app.tar.gz")
    alvo = adicionar(tarball, destino)
    shutil.rmtree(alvo)
    assert not alvo.exists()
