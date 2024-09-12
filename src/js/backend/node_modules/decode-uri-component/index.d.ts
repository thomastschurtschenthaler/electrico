/**
Decodes a Uniform Resource Identifier (URI) component previously created by `encodeURIComponent()`
or by a similar routine.

@param encodedURI - An encoded component of a URI.

@returns The decoded URI component.

@example
```
decodeUriComponent('st%C3%A5le')
//=> 'ståle'
```
*/
export default function decodeUriComponent(encodedURI: string): string;
