import { matchRoutes } from 'react-router-dom'
import { toast } from 'sonner'

import { getSessionToken } from '@/features/auth/session'
import router from 'router/router'

import type { Interceptor } from '@connectrpc/connect'

const loggingInterceptorSkipError = ['AbortError:', 'DOMException:']
export const loggingInterceptor: Interceptor = next => async req => {
  try {
    const result = await next(req)
    console.log(`🔃 to ${req.method.name} `, req.message, result?.message)
    return result
  } catch (e) {
    const error = e
    const errorStr = String(e)

    // only error if it doesn't start with the strings in the array
    if (!loggingInterceptorSkipError.some(s => errorStr.startsWith(s))) {
      console.error(`🚨 to ${req.method.name} `, req.message, error)
    }

    throw error
  }
}

const errorInterceptorSkipError = [
  'TypeError:',
  'AbortError:',
  'DOMException:',
  //extra for local without metering started, TODO consider an alternative rendering of connection errors
  'ConnectError:',
]

export const errorInterceptor: Interceptor = next => async req => {
  try {
    return await next(req)
  } catch (e) {
    const errorStr = String(e)

    if (!errorInterceptorSkipError.some(s => errorStr.startsWith(s))) {
      toast.error(errorStr)
    }
    throw e
  }
}

export const authInterceptor: Interceptor = next => async req => {
  const matchingRoutes = matchRoutes(router.routes, window.location)

  const params = matchingRoutes?.[0]?.params

  const organizationSlug = params?.organizationSlug
  const tenantSlug = params?.tenantSlug

  const token = getSessionToken()

  organizationSlug && req.header.append('x-md-context', `${organizationSlug}/${tenantSlug || ''}`)
  token && req.header.append('Authorization', `Bearer ${token}`)

  const result = await next(req)
  return result
}