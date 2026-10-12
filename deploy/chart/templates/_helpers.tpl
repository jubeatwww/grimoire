{{/* Grimoire's hostname, e.g. grimoire.example.com. */}}
{{- define "grimoire.host" -}}
{{ required (printf "apps.%s.hostname is required (site.yaml)" .Release.Name) .Values.hostname }}
{{- end }}
