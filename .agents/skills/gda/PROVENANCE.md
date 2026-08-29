# gda Skill provenance

- Upstream project: `aigengame/godot-agent`
- Source: https://github.com/aigengame/godot-agent
- Package: https://pypi.org/project/gda/
- Installed package and bundled Skill version: `0.12.0`
- Installed with: `uv tool install gda==0.12.0`
- Skill installation: `gda skill --install --provider codex --scope project`
- Installed on: `2026-08-29` (Asia/Shanghai)
- License: MIT
- Upstream Skill path: `src/gda/skill/SKILL.md`
- Upgrade policy: `MANUAL REVIEW`

The upstream `SKILL.md` is kept unmodified. The PyPI wheel also exposes a `gda-mcp` executable, but Palimpsest does not register, configure, or enable the MCP server. The approved integration is CLI + project Skill only.
