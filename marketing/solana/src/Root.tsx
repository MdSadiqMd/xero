import "./index.css";
import { Composition } from "remotion";
import { LogoReveal } from "./scenes/LogoReveal";
import { AppFlow } from "./scenes/AppFlow";
import { Main, LOGO_FRAMES, APPFLOW_FRAMES, MAIN_FRAMES } from "./Main";

export const RemotionRoot: React.FC = () => {
  return (
    <>
      {/* The full video — watch this one to see everything together. */}
      <Composition
        id="Main"
        component={Main}
        durationInFrames={MAIN_FRAMES}
        fps={30}
        width={1920}
        height={1080}
      />
      {/* Standalone scenes, handy for iterating on one at a time. */}
      <Composition
        id="LogoReveal"
        component={LogoReveal}
        durationInFrames={LOGO_FRAMES}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="AppFlow"
        component={AppFlow}
        durationInFrames={APPFLOW_FRAMES}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
