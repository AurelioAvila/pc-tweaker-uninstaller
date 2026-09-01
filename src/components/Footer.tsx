import { text } from "../i18n";

interface FooterProps {
  openLink: (target: string) => void;
  openRestoreUi: () => void;
}

export function Footer({ openLink, openRestoreUi }: FooterProps) {
  return (
    <footer className="footbar">
      <div className="footbar-left">
        <span className="footbar-family">{text.footer.family}</span>
        <button
          type="button"
          className="footbar-link"
          onClick={() => {
            openLink("pctweaker");
          }}
        >
          {text.footer.pcTweaker}
        </button>
        <span className="footbar-sep" aria-hidden="true">
          ·
        </span>
        <button
          type="button"
          className="footbar-link"
          onClick={() => {
            openLink("promptshield");
          }}
        >
          {text.footer.promptShield}
        </button>
        <span className="footbar-sep" aria-hidden="true">
          ·
        </span>
        <button
          type="button"
          className="footbar-link"
          onClick={() => {
            openLink("privacy");
          }}
        >
          {text.footer.privacy}
        </button>
      </div>
      <div className="footbar-right">
        <button
          type="button"
          className="footbar-link accent"
          title={text.footer.restoreInfo}
          onClick={openRestoreUi}
        >
          {text.footer.openRestore}
        </button>
      </div>
    </footer>
  );
}
