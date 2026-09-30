/**
 * This is intended to be a basic starting point for linting in the extension.
 * It relies on recommended configs out of the box for simplicity, but can be
 * tightened or relaxed as the extension's needs evolve.
 */

module.exports = {
    env: {
        browser: true,
        commonjs: true,
        es6: true,
        node: true,
    },
    extends: [
        'eslint:recommended',
        'plugin:react/recommended',
        'plugin:perfectionist/recommended-natural',
        'plugin:jsdoc/recommended-typescript',
        'prettier',
    ],
    ignorePatterns: [
        '!**/.server',
        '!**/.client',
        'node_modules/**',
        'out/**',
        'test-results/**',
        '*.vsix',
    ],
    overrides: [
        {
            extends: [
                'plugin:react/recommended',
                'plugin:react/jsx-runtime',
                'plugin:react-hooks/recommended',
                'plugin:jsx-a11y/recommended',
            ],
            files: ['**/*.{js,jsx,ts,tsx}'],
            plugins: ['react', 'jsx-a11y'],
            settings: {
                formComponents: ['Form'],
                'import/resolver': {
                    typescript: {},
                },
                linkComponents: [
                    { linkAttribute: 'to', name: 'Link' },
                    { linkAttribute: 'to', name: 'NavLink' },
                ],
                react: {
                    version: '18.2',
                },
            },
        },
        {
            extends: [
                'plugin:@typescript-eslint/recommended',
                'plugin:import/recommended',
                'plugin:import/typescript',
            ],
            files: ['**/*.{ts,tsx}'],
            parser: '@typescript-eslint/parser',
            plugins: ['@typescript-eslint', 'import'],
            rules: {
                '@typescript-eslint/no-explicit-any': 'off',
            },
            settings: {
                'import/internal-regex': '^~/',
                'import/resolver': {
                    node: {
                        extensions: ['.ts', '.tsx'],
                    },
                    typescript: {
                        alwaysTryTypes: true,
                    },
                },
            },
        },
        {
            env: {
                node: true,
            },
            files: ['.eslintrc.cjs', 'vitest.config.mts'],
        },
        {
            extends: ['plugin:playwright/recommended', 'plugin:playwright/playwright-test'],
            files: ['playwright.config.ts', 'tests/e2e/**/*.ts'],
            parser: '@typescript-eslint/parser',
            parserOptions: {
                project: true,
            },
            plugins: ['playwright'],
            rules: {
                'playwright/expect-expect': [
                    'error',
                    {
                        assertFunctionNames: ['expect', 'assertVisible', 'requireBoundingBox'],
                    },
                ],
                'playwright/max-nested-describe': ['error', { max: 2 }],
                'playwright/missing-playwright-await': 'error',
                'playwright/no-commented-out-tests': 'error',
                'playwright/no-conditional-in-test': 'error',
                'playwright/no-element-handle': 'error',
                'playwright/no-eval': 'error',
                'playwright/no-focused-test': 'error',
                'playwright/no-force-option': 'error',
                'playwright/no-networkidle': 'error',
                'playwright/no-raw-locators': 'error',
                'playwright/no-skipped-test': 'error',
                'playwright/no-wait-for-timeout': 'error',
                'playwright/prefer-hooks-in-order': 'error',
                'playwright/prefer-to-have-length': 'error',
                'playwright/prefer-web-first-assertions': 'error',
                'playwright/require-top-level-describe': 'error',
                'playwright/valid-title': [
                    'error',
                    {
                        ignoreTypeOfDescribeName: true,
                    },
                ],
            },
        },
        {
            files: ['**/*.{js,jsx,ts,tsx}'],
            plugins: ['no-unsanitized'],
            rules: {
                'no-unsanitized/method': 'error',
                'no-unsanitized/property': 'error',
            },
        },
        {
            extends: [
                'plugin:functional/recommended',
                'plugin:functional/stylistic',
                'eslint:recommended',
                'plugin:react/recommended',
                'plugin:perfectionist/recommended-natural',
                'plugin:jsdoc/recommended-typescript',
                'prettier',
            ],
            files: ['./app/utils/*.{js,jsx,ts,tsx}'],
            parser: '@typescript-eslint/parser',
            parserOptions: {
                project: true,
            },
            plugins: ['functional'],
            rules: {
                'functional/functional-parameters': 'off',
                'functional/no-expression-statements': 'off',
                'functional/no-loop-statements': 'off',
                'functional/no-return-void': 'off',
            },
        },
    ],
    parserOptions: {
        ecmaFeatures: {
            jsx: true,
        },
        ecmaVersion: 'latest',
        sourceType: 'module',
        tsconfigRootDir: __dirname,
    },
    root: true,
    settings: {
        react: {
            version: '18.2',
        },
    },
};
