export function RoutingNotice({
  hasLikelyVirtualDestination,
}: {
  hasLikelyVirtualDestination: boolean;
}) {
  return (
    <div
      className={hasLikelyVirtualDestination ? 'routing-notice' : 'routing-notice warning'}
      role="note"
    >
      {hasLikelyVirtualDestination ? (
        <>
          A likely virtual playback endpoint is available. Its paired Windows capture endpoint must
          still be selected inside Discord or OBS.
        </>
      ) : (
        <>
          No likely virtual processed destination was found. Physical outputs play through speakers
          or headphones; Discord can select only Windows capture devices. Direct Discord routing
          requires a real virtual capture endpoint or future driver support.
        </>
      )}
    </div>
  );
}
