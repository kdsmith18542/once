# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow these steps:

### 1. **DO NOT** create a public GitHub issue

Security vulnerabilities should be reported privately to avoid exposing users to potential attacks.

### 2. Email us directly

Send an email to: **security@once-lang.org**

Include the following information:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact assessment
- Any suggested fixes or mitigations

### 3. What to expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Initial Assessment**: We will provide an initial assessment within 5 business days
- **Regular Updates**: We will keep you informed of our progress
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days

### 4. Responsible Disclosure

We follow responsible disclosure practices:
- We will not disclose the vulnerability until it's fixed
- We will credit you in our security advisories (unless you prefer to remain anonymous)
- We will work with you to coordinate the public disclosure

## Security Features

The Once language includes several security-focused features:

### Memory Safety
- **Linear Types**: Prevent use-after-free and double-free errors
- **Region Inference**: Automatic memory management with static guarantees
- **Bounds Checking**: Compile-time and runtime array bounds verification

### Concurrency Safety
- **Actor Model**: Isolated concurrency with message passing
- **Effect System**: Track and control side effects
- **Deterministic Scheduling**: Reproducible concurrent execution

### FFI Safety
- **Wasm Component Model**: Secure cross-language interoperability
- **PCC-lite Validation**: Proof-carrying code for verifiable FFI
- **Capability-based Security**: Fine-grained permission system

### Build Security
- **Hermetic Builds**: Reproducible and secure build processes
- **Dependency Verification**: Cryptographic verification of dependencies
- **Sandboxed Execution**: Isolated build and test environments

## Security Best Practices

### For Users
1. **Keep Updated**: Always use the latest version of Once
2. **Review Dependencies**: Audit your project's dependencies regularly
3. **Use Security Features**: Leverage Once's built-in security features
4. **Follow Guidelines**: Adhere to security best practices in your code

### For Contributors
1. **Security Reviews**: All code changes require security review
2. **Testing**: Include security-focused tests in your contributions
3. **Documentation**: Document security implications of new features
4. **Reporting**: Report any security concerns immediately

## Security Advisories

Security advisories are published at: https://github.com/once-lang/once/security/advisories

Subscribe to security notifications to stay informed about security updates.

## Bug Bounty Program

We are considering a bug bounty program for security researchers. More information will be available soon.

## Contact

For security-related questions or concerns:
- **Email**: security@once-lang.org
- **PGP Key**: [Available on our website]
- **Security Team**: @keith/security-team

## Acknowledgments

We thank the security researchers and community members who help keep Once secure by reporting vulnerabilities and contributing to our security efforts.
