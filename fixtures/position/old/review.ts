export interface Review {
  path: string;
  attempts: number;
}

export function summarize(review: Review): string {
  const title = review.path;
  const attempts = Math.max(1, review.attempts);
  return `${title}: ${attempts} attempts`;
}

export const legacy = true;
export const retryDelay = 100;
export const retryLimit = 2;

export const description = "OLD_BEGIN 日本語 café 👩‍💻	old-01 alpha old-02 beta old-03 gamma old-04 delta old-05 epsilon old-06 zeta old-07 eta old-08 theta OLD_ANCHOR old-09 iota old-10 kappa old-11 lambda old-12 mu old-13 nu old-14 xi old-15 omicron old-16 pi OLD_END";

export const context = "SHARED_BEGIN	日本語 café 👩‍💻 shared-01 alpha shared-02 beta shared-03 gamma shared-04 delta shared-05 epsilon shared-06 zeta SHARED_ANCHOR shared-07 eta shared-08 theta shared-09 iota shared-10 kappa shared-11 lambda shared-12 mu SHARED_END";

export const finished = true;
