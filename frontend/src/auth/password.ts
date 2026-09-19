/**
 * The password rules, mirrored from the server (`src/auth/password.rs`) so a
 * form can say what is wrong before asking. The server stays the authority and
 * also refuses common passwords, which the browser cannot check.
 */

export const PASSWORD_MIN_LENGTH = 12
export const PASSWORD_MAX_LENGTH = 128

/** Shown under a new-password field. */
export const PASSWORD_HINT =
  `At least ${PASSWORD_MIN_LENGTH} characters, with an uppercase letter, a lowercase letter, a digit and a symbol.`

/**
 * The first rule `password` breaks, phrased for the person typing it, or null
 * when it passes every rule this side can check.
 */
export function passwordProblem(password: string): string | null {
  const chars = [...password]
  if (chars.length < PASSWORD_MIN_LENGTH) {
    return `Password must be at least ${PASSWORD_MIN_LENGTH} characters.`
  }
  if (chars.length > PASSWORD_MAX_LENGTH) {
    return `Password must be at most ${PASSWORD_MAX_LENGTH} characters.`
  }
  if (!chars.some((c) => c !== c.toLowerCase() && c === c.toUpperCase())) {
    return 'Password must contain an uppercase letter.'
  }
  if (!chars.some((c) => c !== c.toUpperCase() && c === c.toLowerCase())) {
    return 'Password must contain a lowercase letter.'
  }
  if (!chars.some((c) => c >= '0' && c <= '9')) {
    return 'Password must contain a digit.'
  }
  // The server counts anything that is neither alphanumeric nor whitespace.
  if (!chars.some((c) => !/[\p{L}\p{N}\s]/u.test(c))) {
    return 'Password must contain a symbol.'
  }
  return null
}
