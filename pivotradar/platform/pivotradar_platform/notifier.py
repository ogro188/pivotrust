"""Notificador ntfy por activo (topic dedicado por símbolo)."""

from __future__ import annotations

import logging

import requests

from .types import Signal

log = logging.getLogger(__name__)

_TIPO_EMOJI = {"A": "🟢", "B": "🟡", "C": "🟠", "D": "⚪"}


class Notifier:
    def __init__(self, server: str, topic: str, enabled: bool = True):
        self.server = server.rstrip("/")
        self.topic = topic
        self.enabled = enabled

    def _send(self, title: str, message: str, tags: list[str] | None = None):
        if not self.enabled or not self.topic:
            return
        try:
            r = requests.post(f"{self.server}/{self.topic}",
                              json={"title": title, "message": message, "tags": tags or []},
                              timeout=10)
            r.raise_for_status()
        except Exception as e:  # noqa: BLE001
            log.warning("ntfy falló para %s: %s", self.topic, e)

    def signal_new(self, sig: Signal):
        emoji = _TIPO_EMOJI.get(sig.tipo, "")
        dir_txt = "COMPRA" if sig.direction == 1 else "VENTA"
        title = f"{emoji} [{sig.symbol}] {sig.detector} {dir_txt} (Tipo {sig.tipo})"
        lines = [
            f"Entrada: {sig.entry_price}  |  Prob: {sig.hipotesis_prob_min}-{sig.hipotesis_prob_max}%",
            f"Objetivo: {sig.hipotesis_objetivo}",
            f"Contexto estructural: {sig.contexto_estructural:.0f}  |  Tendencia D1: {sig.trend_d1}",
            f"Conf. completa: {sig.conf_completa:.0f}",
        ]
        if sig.hipotesis_causa:
            lines.append(f"⚠️ {sig.hipotesis_causa}")
        if sig.hipotesis_invalidez:
            lines.append(f"❌ {sig.hipotesis_invalidez}")
        self._send(title, "\n".join(lines), tags=["pivotradar", sig.detector])

    def signal_completed(self, sig: Signal):
        ret_4 = sig.retorno[-1] if sig.retorno else 0.0
        dir_txt = "COMPRA" if sig.direction == 1 else "VENTA"
        title = f"⏱️ [{sig.symbol}] {sig.detector} {dir_txt} completada"
        lines = [f"Retorno a 4 velas: {ret_4:.1f} pips", f"MFE: {sig.mfe[-1]:.1f} | MAE: {sig.mae[-1]:.1f}"]
        self._send(title, "\n".join(lines), tags=["pivotradar", "completada"])
