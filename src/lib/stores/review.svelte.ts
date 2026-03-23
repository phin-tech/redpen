interface ReviewSessionState {
  files: string[];
  active: boolean;
}

let state = $state<ReviewSessionState>({
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

export function clearReviewSession() {
  state.files = [];
  state.active = false;
}
