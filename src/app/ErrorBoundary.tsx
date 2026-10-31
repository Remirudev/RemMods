import { Component, type ErrorInfo, type ReactNode } from 'react'
import { logError } from '../lib/log/frontend-log'
import { ErrorFallback } from './ErrorFallback'
import type { ErrorBoundaryProps, ErrorBoundaryState } from './types'

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null }

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { error: error instanceof Error ? error : new Error(String(error)) }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    const area = this.props.area ?? 'app'
    logError(`render error in ${area}: ${error.message}\n${info.componentStack ?? ''}`)
  }

  private readonly reset = (): void => {
    this.setState({ error: null })
  }

  render(): ReactNode {
    const { error } = this.state
    if (!error) return this.props.children
    return <ErrorFallback area={this.props.area} error={error} onReset={this.reset} />
  }
}
