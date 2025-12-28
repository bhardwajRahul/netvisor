import pkg from 'ipaddr.js';
import { validate } from 'email-validator';

const { isValid, isValidCIDR, parse, parseCIDR } = pkg;

// ============================================================================
// TanStack Form Validators
// These return `string | undefined` - undefined means valid, string is error
// ============================================================================

/** Required field validator */
export function required(value: string | null | undefined): string | undefined {
	return !value?.trim() ? 'This field is required' : undefined;
}

/** Email format validator */
export function email(value: string): string | undefined {
	if (!value) return undefined;
	return !validate(value) ? 'Please enter a valid email address' : undefined;
}

/** Maximum length validator */
export function max(length: number): (value: string) => string | undefined {
	return (value: string) => {
		if (!value) return undefined;
		return value.length > length ? `Must be less than ${length} characters` : undefined;
	};
}

/** Minimum length validator */
export function min(length: number): (value: string) => string | undefined {
	return (value: string) => {
		if (!value) return undefined;
		return value.length < length ? `Must be at least ${length} characters` : undefined;
	};
}

/** CIDR notation validator */
export function cidrNotation(value: string): string | undefined {
	if (!value) return undefined;
	return !isValidCIDR(value) ? 'Invalid CIDR notation (e.g., 192.168.1.0/24)' : undefined;
}

/** IP address validator */
export function ipAddressFormat(value: string): string | undefined {
	if (!value) return undefined;
	return !isValid(value) ? 'Invalid IP address format' : undefined;
}

/** IP address within CIDR range validator */
export function ipAddressInCidrFormat(cidr: string): (value: string) => string | undefined {
	return (value: string) => {
		if (!value) return undefined;
		if (!isValidCIDR(cidr)) return `${cidr} is not valid CIDR notation`;
		if (!isValid(value)) return 'Invalid IP address format';
		if (!parse(value).match(parseCIDR(cidr))) {
			return `IP must be within ${cidr}`;
		}
		return undefined;
	};
}

/** MAC address validator */
export function macAddress(value: string): string | undefined {
	if (!value) return undefined;
	const macRegex = /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/;
	return !macRegex.test(value) ? 'Invalid MAC address format' : undefined;
}

/** MAC address format validator (alias for macAddress) */
export const macFormat = macAddress;

/** Hostname validator */
export function hostnameFormat(value: string): string | undefined {
	if (!value) return undefined;
	const hostnameRegex =
		/^([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)*[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$/;
	return !hostnameRegex.test(value.trim()) ? 'Please enter a valid hostname' : undefined;
}

/** Port range validator (1-65535) */
export function port(value: number | string): string | undefined {
	if (!value && value !== 0) return undefined;
	const portNum = typeof value === 'string' ? parseInt(value) : value;
	return !Number.isInteger(portNum) || portNum < 1 || portNum > 65535
		? 'Port must be between 1 and 65535'
		: undefined;
}

/** Port range validation (alias for port) */
export const portRangeValidation = port;

/** URL format validator */
export function url(value: string): string | undefined {
	if (!value) return undefined;
	try {
		const parsed = new URL(value);
		// Require hostname to have at least one dot (e.g., example.com) or be localhost
		const hostname = parsed.hostname;
		if (hostname !== 'localhost' && !hostname.includes('.')) {
			return 'Please enter a valid URL with a domain (e.g., https://example.com)';
		}
		return undefined;
	} catch {
		return 'Please enter a valid URL';
	}
}

/** Numeric value validator */
export function numeric(value: string): string | undefined {
	if (!value) return undefined;
	return isNaN(Number(value)) ? 'Must be a number' : undefined;
}

/** Password complexity validator */
export function password(value: string): string | undefined {
	if (!value) return undefined;
	const hasUppercase = /[A-Z]/.test(value);
	const hasLowercase = /[a-z]/.test(value);
	const hasNumber = /[0-9]/.test(value);
	const longEnough = value.length >= 10;
	if (!longEnough) return 'Password must be at least 10 characters';
	if (!hasUppercase || !hasLowercase || !hasNumber) {
		return 'Password must contain uppercase, lowercase, and number';
	}
	return undefined;
}

/** Confirm password match validator */
export function confirmPasswordMatch(
	getPassword: () => string
): (value: string) => string | undefined {
	return (value: string) => {
		const passwordValue = getPassword();
		if (!passwordValue && !value) return undefined;
		if (passwordValue && !value) return 'Please confirm your password';
		if (value !== passwordValue) return 'Passwords do not match';
		return undefined;
	};
}

/** Pattern matcher validator */
export function pattern(regex: RegExp, message: string): (value: string) => string | undefined {
	return (value: string) => {
		if (!value) return undefined;
		return !regex.test(value) ? message : undefined;
	};
}