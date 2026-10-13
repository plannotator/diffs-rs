export interface Review {
  path: string;
  attempts: number;
}

export function summarize(review: Review): string {
  const title = review.path.trim();
  const attempts = Math.max(0, review.attempts);
  return `${title}: ${attempts} attempts remaining`;
}

export const retryLimit = 5;

export const description = "NEW_BEGIN 日本語 café 👩‍💻	new-01 alpha new-02 beta new-03 gamma new-04 delta new-05 epsilon new-06 zeta new-07 eta new-08 theta NEW_ANCHOR new-09 iota new-10 kappa new-11 lambda new-12 mu new-13 nu new-14 xi new-15 omicron new-16 pi new-17 rho new-18 sigma new-19 tau new-20 upsilon new-21 phi new-22 chi new-23 psi new-24 omega NEW_END";

export const context = "SHARED_BEGIN	日本語 café 👩‍💻 shared-01 alpha shared-02 beta shared-03 gamma shared-04 delta shared-05 epsilon shared-06 zeta SHARED_ANCHOR shared-07 eta shared-08 theta shared-09 iota shared-10 kappa shared-11 lambda shared-12 mu SHARED_END";

export const finished = true;
