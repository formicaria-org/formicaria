/** The tag a called-off meeting carries — set by the calendars (`fm_core::calendar::CANCELLED_TAG`) and
 *  by the assistant when an email calls a meeting off. **Kept out of the renderers**, which stay free
 *  of scheduling literals: they ask this, and draw a called-off meeting struck through rather than
 *  hiding it — the date stays readable as the meeting that did not happen (marked, never deleted). */
export const CALLED_OFF_TAG = 'cancelled';

export const calledOff = (card: { tags?: string[] }): boolean =>
  (card.tags ?? []).includes(CALLED_OFF_TAG);
