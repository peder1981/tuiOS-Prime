from pathlib import Path

import pytest

from tuiosapps.manifest import ErroManifesto, carregar_manifesto


def _gravar(dir_app: Path, texto: str) -> Path:
    dir_app.mkdir(parents=True, exist_ok=True)
    (dir_app / "tuios-app.toml").write_text(texto, encoding="utf-8")
    return dir_app


def test_manifesto_valido(app_valido: Path):
    m = carregar_manifesto(app_valido)
    assert m.nome == "ola-tuios"
    assert m.versao == "1.0.0"
    assert m.entry == "ola.prw"
    assert m.categoria == "exemplo"
    assert m.autor is None
    assert m.path == app_valido


def test_sem_manifesto_erro(tmp_path: Path):
    with pytest.raises(ErroManifesto, match="manifesto nao encontrado"):
        carregar_manifesto(tmp_path)


def test_toml_invalido(tmp_path: Path):
    _gravar(tmp_path, "nome = [quebrado")
    with pytest.raises(ErroManifesto, match="TOML invalido"):
        carregar_manifesto(tmp_path)


def test_campo_obrigatorio_ausente(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('descricao = "Primeiro app tuiOS"\n', ""), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="obrigatorio ausente: descricao"):
        carregar_manifesto(app_valido)


def test_chave_desconhecida_erro(app_valido: Path):
    with app_valido.joinpath("tuios-app.toml").open("a", encoding="utf-8") as f:
        f.write('descricao_extra = "typo"\n')  # bare-key ASCII: TOML rejeita ç
    with pytest.raises(ErroManifesto, match="chave desconhecida"):
        carregar_manifesto(app_valido)


def test_nome_fora_do_slug(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('nome = "ola-tuios"', 'nome = "Meu App!"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="nome"):
        carregar_manifesto(app_valido)


def test_versao_nao_semver(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('versao = "1.0.0"', 'versao = "1.0"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="versao"):
        carregar_manifesto(app_valido)


def test_descricao_grande_demais(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('descricao = "Primeiro app tuiOS"', f'descricao = "{"x" * 121}"'),
        encoding="utf-8",
    )
    with pytest.raises(ErroManifesto, match="descricao"):
        carregar_manifesto(app_valido)


def test_entry_inexistente(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "sumiu.prw"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_entry_absoluto_rejeitado(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "/etc/passwd"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_entry_extensao_errada(app_valido: Path):
    (app_valido / "nota.txt").write_text("x", encoding="utf-8")
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('entry = "ola.prw"', 'entry = "nota.txt"'), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="entry"):
        carregar_manifesto(app_valido)


def test_campo_nao_texto(app_valido: Path):
    texto = (app_valido / "tuios-app.toml").read_text(encoding="utf-8")
    (app_valido / "tuios-app.toml").write_text(
        texto.replace('versao = "1.0.0"', "versao = 1"), encoding="utf-8"
    )
    with pytest.raises(ErroManifesto, match="versao"):
        carregar_manifesto(app_valido)
