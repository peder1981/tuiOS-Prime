"""Menu TUI (dialog) — camada fina sobre os módulos do tuios-apps (R14, R15)."""
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from .discovery import App, buscar, descobrir, roots
from .manifest import ErroManifesto
from .package import ErroPacote, adicionar
from .runner import AdvplcAusente, rodar


class DialogAusente(Exception):
    """Binário dialog fora do PATH/inválido — o CLI converte em exit 4."""


def _resolver_dialog() -> str:
    alvo = os.environ.get("TUIOS_DIALOG", "dialog")
    if os.path.sep in alvo:
        if os.path.isfile(alvo) and os.access(alvo, os.X_OK):
            return alvo
    else:
        achado = shutil.which(alvo)
        if achado:
            return achado
    raise DialogAusente(
        "dialog nao encontrado no PATH (instale dialog ou defina TUIOS_DIALOG)"
    )


def _dialog(alvo: str, args: list[str]) -> tuple[int, str]:
    """Uma invocação do dialog; devolve (rc, stdout)."""
    r = subprocess.run([alvo, *args], capture_output=True, text=True, shell=False)
    return r.returncode, r.stdout.strip()


def _msgbox(alvo: str, texto: str) -> None:
    _dialog(alvo, ["--msgbox", texto, "0", "0"])


def _raiz_usuario() -> Path:
    return next(c for o, c in roots() if o == "usuario")


def _tela_listar(alvo: str) -> None:
    apps = descobrir()
    if not apps:
        _msgbox(alvo, "nenhum app instalado")
        return
    itens: list[str] = []
    for a in apps:
        itens += [a.nome, f"{a.versao} — {a.descricao} [{a.origem}]"]
    itens += ["voltar", "Voltar"]
    rc, escolha = _dialog(
        alvo, ["--stdout", "--menu", "Apps instalados", "0", "0", "0", *itens]
    )
    if rc != 0 or escolha == "voltar":
        return
    app = buscar(escolha)
    if app is None:
        _msgbox(alvo, f"app nao encontrado: {escolha}")
        return
    _tela_app(alvo, app)


def _tela_app(alvo: str, app: App) -> None:
    while True:
        rc, acao = _dialog(
            alvo,
            [
                "--stdout", "--menu", f"{app.nome} {app.versao}", "0", "0", "0",
                "detalhes", "Ver detalhes",
                "rodar", "Rodar agora",
                "remover", "Remover",
                "voltar", "Voltar",
            ],
        )
        if rc != 0 or acao == "voltar":
            return
        if acao == "detalhes":
            _msgbox(
                alvo,
                f"nome: {app.nome}\nversao: {app.versao}\n"
                f"descricao: {app.descricao}\norigem: {app.origem}\n"
                f"entry: {app.entry}\npath: {app.path}",
            )
        elif acao == "rodar":
            try:
                codigo = rodar(app, [])
            except AdvplcAusente as exc:
                _msgbox(alvo, str(exc))
                continue
            _msgbox(alvo, f"exit code: {codigo}")
        elif acao == "remover":
            if app.origem != "usuario":
                _msgbox(
                    alvo,
                    f"app de sistema — remova pela CLI: "
                    f"tuios-apps remover {app.nome} --sistema",
                )
                continue
            rc2, _ = _dialog(alvo, ["--yesno", f"Remover {app.nome}?", "0", "0"])
            if rc2 != 0:
                continue
            try:
                shutil.rmtree(app.path)
            except OSError as exc:
                _msgbox(alvo, f"erro ao remover: {exc}")
                continue
            _msgbox(alvo, f"removido: {app.path}")
            return


def _tela_instalar(alvo: str) -> None:
    rc, caminho = _dialog(
        alvo, ["--stdout", "--fselect", str(Path.home()) + "/", "0", "0"]
    )
    if rc != 0 or not caminho:
        return
    try:
        instalado = adicionar(Path(caminho), _raiz_usuario())
    except (ErroPacote, ErroManifesto) as exc:
        _msgbox(alvo, f"erro: {exc}")
        return
    _msgbox(alvo, f"instalado: {instalado}")


def rodar_menu() -> int:
    """Loop principal do menu; devolve 0 ao sair (erros viram msgbox)."""
    alvo = _resolver_dialog()
    while True:
        rc, escolha = _dialog(
            alvo,
            [
                "--stdout", "--menu",
                "tuios-apps — apps AdvPL de primeira classe",
                "0", "0", "0",
                "listar", "Listar apps instalados",
                "instalar", "Instalar pacote .tar.gz",
                "sair", "Sair",
            ],
        )
        if rc != 0 or escolha == "sair":
            return 0
        if escolha == "listar":
            _tela_listar(alvo)
        elif escolha == "instalar":
            _tela_instalar(alvo)
