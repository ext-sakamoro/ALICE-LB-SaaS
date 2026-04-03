import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
export function middleware(request: NextRequest) {
  const isAuth = request.cookies.has('sb-access-token') || request.cookies.has('sb-refresh-token');
  if (request.nextUrl.pathname.startsWith('/dashboard') && !isAuth) return NextResponse.redirect(new URL('/auth/login', request.url));
  if ((request.nextUrl.pathname === '/auth/login' || request.nextUrl.pathname === '/auth/register') && isAuth) return NextResponse.redirect(new URL('/dashboard', request.url));
  return NextResponse.next();
}
export const config = { matcher: ['/dashboard/:path*', '/auth/login', '/auth/register'] };
