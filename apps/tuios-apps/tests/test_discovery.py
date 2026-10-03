from pathlib import Path

import pytest

from tuiosapps.discovery import buscar, descobrir, roots


@pytest.fixture
def roots_tmp(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    sistema = tmp_path / "sistema"
    usuario = tmp_path / "usuario"
    sistema.mkdir()
    usuario.mkdir()
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", str(sistema))
    monkeypatch.setenv("TUIOS_APPS_USER", str(usuario))
    return sistema, usuario


def _copiar_app(origem: Path, destino_root: Path) -> Path:
    destino = destino_root / origem.name
    destino.mkdir()
    for arquivo in origem.iterdir():
        (destino / arquivo.name).write_bytes(arquivo.read_bytes())
    return destino


def _silencio(_msg: str) -> None:
    pass


def test_roots_por_env(roots_tmp):
    sistema, usuario = roots_tmp
    assert roots() == [("sistema", sistema), ("usuario", usuario)]


def test_descobre_app_do_usuario(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    apps = descobrir(avisar=_silencio)
    assert [a.nome for a in apps] == ["ola-tuios"]
    assert apps[0].origem == "usuario"
    assert apps[0].entry == "ola.prw"


def test_descobre_app_do_sistema(roots_tmp, app_valido: Path):
    sistema, _ = roots_tmp
    _copiar_app(app_valido, sistema)
    apps = descobrir(avisar=_silencio)
    assert apps[0].origem == "sistema"


def test_pasta_sem_manifesto_pula_com_aviso(roots_tmp, capsys):
    _, usuario = roots_tmp
    (usuario / "lixo").mkdir()
    apps = descobrir()
    assert apps == []
    assert "pulada" in capsys.readouterr().err


def test_manifesto_invalido_pula_com_aviso(roots_tmp, capsys):
    _, usuario = roots_tmp
    (usuario / "quebrado").mkdir()
    (usuario / "quebrado" / "tuios-app.toml").write_text("nome = [x", encoding="utf-8")
    apps = descobrir()
    assert apps == []
    assert "app pulado" in capsys.readouterr().err


def test_usuario_sombrea_sistema(roots_tmp, app_valido: Path):
    sistema, usuario = roots_tmp
    _copiar_app(app_valido, sistema)
    novo = _copiar_app(app_valido, usuario)
    (novo / "tuios-app.toml").write_text(
        (novo / "tuios-app.toml").read_text(encoding="utf-8").replace("1.0.0", "2.0.0"),
        encoding="utf-8",
    )
    apps = descobrir(avisar=_silencio)
    assert len(apps) == 1
    assert apps[0].origem == "usuario"
    assert apps[0].versao == "2.0.0"


def test_nome_duplicado_na_mesma_raiz_mantem_o_primeiro(roots_tmp, app_valido: Path):
    sistema, _ = roots_tmp
    _copiar_app(app_valido, sistema)
    # segunda pasta com o MESMO nome de app (manifesto nome = "ola-tuios")
    segunda = sistema / "alias"
    segunda.mkdir()
    for arquivo in app_valido.iterdir():
        (segunda / arquivo.name).write_bytes(arquivo.read_bytes())
    avisos: list[str] = []
    apps = descobrir(avisar=avisos.append)
    assert len(apps) == 1
    assert any("duplicado" in a for a in avisos)


def test_ordenado_por_nome(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    outro = usuario / "ze-app"
    outro.mkdir()
    (outro / "tuios-app.toml").write_text(
        'nome = "ze-app"\nversao = "0.1.0"\ndescricao = "Z"\nentry = "z.prw"\n',
        encoding="utf-8",
    )
    (outro / "z.prw").write_text("Return", encoding="utf-8")
    assert [a.nome for a in descobrir(avisar=_silencio)] == ["ola-tuios", "ze-app"]


def test_raiz_inexistente_ignorada(monkeypatch):
    monkeypatch.setenv("TUIOS_APPS_SYSTEM", "/nao/existe/a")
    monkeypatch.setenv("TUIOS_APPS_USER", "/nao/existe/b")
    assert descobrir(avisar=_silencio) == []


def test_buscar_encontra_e_retorna_none(roots_tmp, app_valido: Path):
    _, usuario = roots_tmp
    _copiar_app(app_valido, usuario)
    assert buscar("ola-tuios", avisar=_silencio) is not None
    assert buscar("nao-existe", avisar=_silencio) is None
