interface ReviewSessionState {
  id: string | null;
  files: string[];
  active: boolean;
}

let state = $state<ReviewSessionState>({
  id: null,
  files: [],
  active: false,
});

export function getReviewSession() {
  return state;
}

export function addReviewFile(filePath: string) {
  if (!state.files.includes(filePath)) {
    state.files = [...state.files, filePath];
    state.active = true;
  }
}

export function activateReviewSession(sessionId: string | null, files: string[]) {
  state.id = sessionId;
  state.files = [...files];
  state.active = files.length > 0;
}

export function clearReviewSession() {
  state.id = null;
  state.files = [];
  state.active = false;
}

export function resetReviewSessionForTests() {
  clearReviewSession();
}
