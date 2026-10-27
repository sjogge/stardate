import type { ParamMatcher } from '@sveltejs/kit';

/** Route ids such as `/jobs/[jobId=integer]`: anything else is a 404 instead of a bad API call */
export const match: ParamMatcher = (param) => /^[1-9]\d*$/.test(param);
