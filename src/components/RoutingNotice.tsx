import type { ExternalAudioRoute } from '../types/audio';

export function RoutingNotice({ route }: { route: ExternalAudioRoute | null }) {
  return (
    <div className={route?.captureDevice ? 'routing-notice' : 'routing-notice warning'} role="note">
      {route?.captureDevice ? (
        <>
          <strong>Receiving-application microphone</strong>
          <p>In Discord, OBS, or another receiving application, select:</p>
          <code>{route.captureDevice.name}</code>
          <p>
            Playback active means Mam Voice Changer is writing to the virtual playback endpoint. It
            does not prove that Discord or another application is consuming this capture endpoint.
          </p>
        </>
      ) : (
        <>
          <strong>No virtual audio route is available.</strong>
          <p>
            Install or enable a compatible Windows virtual audio device, then refresh devices.
            Physical speakers are not selected automatically.
          </p>
        </>
      )}
    </div>
  );
}
