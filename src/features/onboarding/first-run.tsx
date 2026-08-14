import * as React from "react";
import {
  AudioLines,
  Brain,
  CheckCircle2,
  Cloud,
  Download,
  Loader2,
  ShieldCheck,
  Sparkles,
  Zap,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Switch } from "@/shared/ui/switch";
import { cn, formatBytes } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import {
  ensureWhisperModel,
  onWhisperDownloadProgress,
  setProviderKey,
  whisperModelStatus,
} from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { isMac, keychainName } from "@/shared/lib/platform";
import { PermissionsScreen } from "./permissions-screen";
import type { WhisperDownloadProgress } from "@/shared/lib/ipc";
import type { WhisperModel } from "@/shared/types/WhisperModel";

type Transcriber = "local_whisper" | "openai";

type Step = "permissions" | "transcriber" | "features";

export function FirstRunConductor({ onFinish }: { onFinish: () => void }) {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);

  const [step, setStep] = React.useState<Step>("permissions");
  const [transcriber, setTranscriber] = React.useState<Transcriber>(
    (settings?.transcriber as Transcriber) ?? "local_whisper"
  );
  const [openaiKey, setOpenaiKey] = React.useState("");
  const [savingKey, setSavingKey] = React.useState(false);

  const [autoTranscribe, setAutoTranscribe] = React.useState(true);
  const [stripSilence, setStripSilence] = React.useState(true);

  // Whisper model download state
  const [downloading, setDownloading] = React.useState(false);
  const [progress, setProgress] = React.useState<WhisperDownloadProgress | null>(null);
  const [modelDownloaded, setModelDownloaded] = React.useState(false);

  const finish = React.useCallback(async () => {
    if (transcriber === "openai" && openaiKey.trim().length > 0) {
      try {
        setSavingKey(true);
        await setProviderKey("openai", openaiKey.trim());
      } catch (e) {
        console.error("set_provider_key:", e);
        toast.error("Could not save OpenAI key", { description: humanizeError(e) });
        setSavingKey(false);
        return;
      }
      setSavingKey(false);
    }
    if (!settings) return;
    try {
      await saveSettings({
        ...settings,
        transcriber,
        auto_transcribe_enabled: autoTranscribe,
        auto_vad_enabled: stripSilence,
        onboarding_completed: true,
      });
      toast.success("You're set up", {
        description: `Press ${isMac() ? "Cmd" : "Ctrl"}-R any time to start recording.`,
      });
      onFinish();
    } catch (e) {
      console.error("update settings on first-run finish:", e);
      toast.error("Could not save preferences", { description: humanizeError(e) });
    }
  }, [
    openaiKey,
    settings,
    transcriber,
    autoTranscribe,
    stripSilence,
    saveSettings,
    onFinish,
  ]);

  // Check if the model is already downloaded
  React.useEffect(() => {
    if (transcriber !== "local_whisper") return;
    let cancelled = false;
    (async () => {
      try {
        const status = await whisperModelStatus();
        if (!cancelled && status?.present) {
          setModelDownloaded(true);
        }
      } catch {}
    })();
    return () => {
      cancelled = true;
    };
  }, [transcriber]);

  // Listen for download progress
  React.useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;
    if (transcriber === "local_whisper") {
      (async () => {
        const unlisten = await onWhisperDownloadProgress<WhisperDownloadProgress>(
          (payload) => {
            if (cancelled) return;
            setProgress(payload);
          }
        );
        if (cancelled) {
          unlisten();
        } else {
          unlistenFn = unlisten;
        }
      })();
    }
    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
    };
  }, [transcriber]);

  const handleDownload = async () => {
    setDownloading(true);
    setProgress(null);
    try {
      const next = await ensureWhisperModel(
        (settings?.local_whisper_model ?? "small") as WhisperModel
      );
      setModelDownloaded(true);
      toast.success("Whisper model ready", {
        description: `${formatBytes(Number(next.bytes_on_disk ?? 0n))} on disk.`,
      });
    } catch (e) {
      console.error("ensure_whisper_model:", e);
      toast.error("Could not download model", { description: humanizeError(e) });
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  if (!settings) return null;

  if (step === "permissions") {
    return <PermissionsScreen onContinue={() => setStep("transcriber")} />;
  }

  if (step === "transcriber") {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-8 py-12">
        <header data-drag="" className="select-none">
          <div className="flex items-center gap-3">
            <Sparkles className="h-6 w-6 text-primary" />
            <h1 className="font-serif text-4xl font-medium tracking-tight">
              Welcome to Meety
            </h1>
          </div>
          <p className="mt-2 text-sm text-muted-foreground">
            One last thing — pick how you want transcripts to happen.
          </p>
        </header>

        <Card>
          <CardContent className="flex flex-col gap-4 py-5">
            <div className="flex items-center gap-2">
              <Brain className="h-4 w-4 text-muted-foreground" />
              <h2 className="font-medium">Pick transcription</h2>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <TranscriberChoice
                selected={transcriber === "local_whisper"}
                onClick={() => setTranscriber("local_whisper")}
                icon={ShieldCheck}
                title="Local Whisper"
                detail="Runs on your device. Free. No network. Slower on first run while the model downloads."
              />
              <TranscriberChoice
                selected={transcriber === "openai"}
                onClick={() => setTranscriber("openai")}
                icon={Cloud}
                title="OpenAI Whisper"
                detail="Cloud API. Faster on long meetings. Needs your OpenAI key."
              />
            </div>
            {transcriber === "openai" ? (
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-xs text-muted-foreground">
                  OpenAI API key (stored in {keychainName()}, never on disk in plain
                  text)
                </span>
                <input
                  type="password"
                  value={openaiKey}
                  onChange={(e) => setOpenaiKey(e.target.value)}
                  placeholder="sk-..."
                  autoComplete="off"
                  spellCheck={false}
                  className="rounded-md border border-border bg-background px-3 py-1.5 font-mono text-xs outline-none focus:border-ring"
                />
              </label>
            ) : null}
          </CardContent>
        </Card>

        <div className="flex items-center justify-end">
          <Button onClick={() => setStep("features")} className="gap-2">
            Continue
          </Button>
        </div>
      </div>
    );
  }

  // Step: features
  const totalBytes = progress?.total ?? 0;
  const percent =
    progress && totalBytes > 0
      ? Math.min(100, Math.floor((progress.downloaded / totalBytes) * 100))
      : null;

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-8 py-12">
      <header data-drag="" className="select-none">
        <div className="flex items-center gap-3">
          <Sparkles className="h-6 w-6 text-primary" />
          <h1 className="font-serif text-4xl font-medium tracking-tight">
            Almost done
          </h1>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          A few optional tweaks to get the best results.
        </p>
      </header>

      {/* Auto-transcribe toggle */}
      <Card>
        <CardContent className="py-5">
          <div className="flex items-start justify-between gap-6">
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Zap className="h-4 w-4 text-muted-foreground" />
                Auto-transcribe after recording
                <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
                  Recommended
                </span>
              </div>
              <p className="max-w-md text-xs text-muted-foreground">
                Transcribe automatically as soon as a recording stops. Turn this off if
                you prefer to transcribe manually from the Library.
              </p>
            </div>
            <Switch
              checked={autoTranscribe}
              onCheckedChange={setAutoTranscribe}
              className="mt-1"
              aria-label="Auto-transcribe after recording"
            />
          </div>
        </CardContent>
      </Card>

      {/* VAD / strip silence toggle */}
      <Card>
        <CardContent className="py-5">
          <div className="flex items-start justify-between gap-6">
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Zap className="h-4 w-4 text-muted-foreground" />
                Strip silence before transcription
                <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
                  Recommended
                </span>
              </div>
              <p className="max-w-md text-xs text-muted-foreground">
                Removes silent stretches before sending to the transcriber. This
                prevents the model from hallucinating over silence and cuts cloud upload
                size on meetings with long pauses.
              </p>
            </div>
            <Switch
              checked={stripSilence}
              onCheckedChange={setStripSilence}
              className="mt-1"
              aria-label="Strip silence before transcription"
            />
          </div>
        </CardContent>
      </Card>

      {/* Model download (local Whisper only) */}
      {transcriber === "local_whisper" && !modelDownloaded ? (
        <Card>
          <CardContent className="py-5">
            <div className="flex items-center justify-between gap-4">
              <div className="space-y-1">
                <div className="flex items-center gap-2 text-sm font-medium">
                  <Download className="h-4 w-4 text-muted-foreground" />
                  Download Whisper model
                </div>
                <p className="max-w-md text-xs text-muted-foreground">
                  Meety needs a local Whisper model to transcribe your recordings. Tiny
                  (~75 MB) is the fastest; Small (~466 MB) is the best balance of
                  quality and speed. You can change the model later in Settings.
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={handleDownload}
                disabled={downloading}
                className="h-8 shrink-0 gap-1.5"
              >
                {downloading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Download className="h-3.5 w-3.5" />
                )}
                {downloading ? "Downloading…" : "Download Small"}
              </Button>
            </div>
            {downloading && progress ? (
              <div
                className="mt-3 flex flex-col gap-1"
                role="status"
                aria-live="polite"
              >
                <span className="font-mono text-2xs text-muted-foreground">
                  {formatBytes(progress.downloaded)}
                  {progress.total ? ` / ${formatBytes(progress.total)}` : ""}
                  {percent !== null ? ` · ${percent}%` : ""}
                </span>
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-secondary">
                  <div
                    className="h-full w-full origin-left bg-primary transition-transform"
                    style={{ transform: `scaleX(${(percent ?? 0) / 100})` }}
                  />
                </div>
              </div>
            ) : null}
          </CardContent>
        </Card>
      ) : null}

      {/* Model already downloaded */}
      {transcriber === "local_whisper" && modelDownloaded ? (
        <Card>
          <CardContent className="py-5">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="h-4 w-4 text-emerald-500" />
              <span className="text-sm font-medium text-emerald-600 dark:text-emerald-400">
                Whisper model ready
              </span>
            </div>
          </CardContent>
        </Card>
      ) : null}

      <div
        className={cn(
          "flex items-center justify-between rounded-lg border border-primary/30 bg-primary/5 p-4"
        )}
      >
        <div className="flex items-center gap-2">
          <CheckCircle2 className="h-4 w-4 text-primary" />
          <p className="text-sm">
            You can change everything later in Preferences ({isMac() ? "Cmd-" : "Ctrl-"}
            ,).
          </p>
        </div>
        <Button
          onClick={finish}
          disabled={
            savingKey ||
            (transcriber === "local_whisper" && !modelDownloaded && downloading)
          }
          className="gap-2"
        >
          <AudioLines className="h-4 w-4" />
          I&apos;m ready
        </Button>
      </div>
    </div>
  );
}

interface ChoiceProps {
  selected: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  detail: string;
}

function TranscriberChoice({
  selected,
  onClick,
  icon: Icon,
  title,
  detail,
}: ChoiceProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={cn(
        "flex flex-col items-start gap-2 rounded-md border p-3 text-left transition-colors",
        selected
          ? "border-primary bg-primary/5 ring-1 ring-primary/30"
          : "border-border bg-card hover:bg-muted/40"
      )}
    >
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">{title}</span>
        {selected ? <CheckCircle2 className="h-3.5 w-3.5 text-primary" /> : null}
      </div>
      <p className="text-xs text-muted-foreground">{detail}</p>
    </button>
  );
}
