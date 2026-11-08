export type Vec3=[number,number,number];
export type Quat=[number,number,number,number];
export type DieMesh={vertices:Vec3[];faces:number[][];normals:Vec3[]};
export const add=(a:Vec3,b:Vec3):Vec3=>[a[0]+b[0],a[1]+b[1],a[2]+b[2]];
export const sub=(a:Vec3,b:Vec3):Vec3=>[a[0]-b[0],a[1]-b[1],a[2]-b[2]];
export const scale=(a:Vec3,n:number):Vec3=>[a[0]*n,a[1]*n,a[2]*n];
export const dot=(a:Vec3,b:Vec3)=>a[0]*b[0]+a[1]*b[1]+a[2]*b[2];
export const cross=(a:Vec3,b:Vec3):Vec3=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
export const unit=(a:Vec3):Vec3=>scale(a,1/(Math.hypot(...a)||1));
export const center=(vertices:Vec3[]):Vec3=>scale(vertices.reduce(add,[0,0,0] as Vec3),1/vertices.length);
export function hull(points:Vec3[]):DieMesh{
 const radius=Math.max(...points.map(p=>Math.hypot(...p))),vertices=points.map(p=>scale(p,1/radius)),seen=new Set<string>(),faces:number[][]=[],normals:Vec3[]=[];
 for(let i=0;i<vertices.length;i++)for(let j=i+1;j<vertices.length;j++)for(let k=j+1;k<vertices.length;k++){
  let normal=cross(sub(vertices[j],vertices[i]),sub(vertices[k],vertices[i]));if(Math.hypot(...normal)<1e-7)continue;normal=unit(normal);
  const values=vertices.map(p=>dot(normal,sub(p,vertices[i])));if(values.some(v=>v>1e-6)&&values.some(v=>v< -1e-6))continue;if(dot(normal,vertices[i])<0)normal=scale(normal,-1);
  const indices=values.flatMap((v,index)=>Math.abs(v)<1e-6?[index]:[]),key=indices.join(",");if(seen.has(key))continue;seen.add(key);
  const c=center(indices.map(index=>vertices[index])),u=unit(sub(vertices[indices[0]],c)),v=cross(normal,u);
  indices.sort((a,b)=>Math.atan2(dot(sub(vertices[a],c),v),dot(sub(vertices[a],c),u))-Math.atan2(dot(sub(vertices[b],c),v),dot(sub(vertices[b],c),u)));faces.push(indices);normals.push(normal);
 }return {vertices,faces,normals};
}
function dual(mesh:DieMesh):DieMesh{return hull(mesh.faces.map((face,i)=>scale(mesh.normals[i],1/dot(mesh.normals[i],mesh.vertices[face[0]]))));}
const cache=new Map<number,DieMesh>();
export function dieMesh(sides:number):DieMesh|null{
 if(cache.has(sides))return cache.get(sides)!;let points:Vec3[]=[];let mesh:DieMesh;
 if(sides===4)points=[[1,1,1],[1,-1,-1],[-1,1,-1],[-1,-1,1]];
 else if(sides===6){for(const x of [-1,1])for(const y of [-1,1])for(const z of [-1,1])points.push([x,y,z]);}
 else if(sides===8)points=[[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]];
 else if(sides===20||sides===12){const p=(1+Math.sqrt(5))/2;for(const a of [-1,1])for(const b of [-p,p])points.push([0,a,b],[a,b,0],[b,0,a]);}
 else if(sides===10){for(let i=0;i<5;i++){const angle=i*Math.PI*2/5;points.push([Math.cos(angle),Math.sin(angle),.5],[Math.cos(angle+Math.PI/5),Math.sin(angle+Math.PI/5),-.5]);}mesh=dual(hull(points));cache.set(sides,mesh);return mesh;}
 else return null;
 mesh=hull(points);if(sides===12)mesh=dual(mesh);cache.set(sides,mesh);return mesh;
}
export function multiply(a:Quat,b:Quat):Quat{return [a[3]*b[0]+a[0]*b[3]+a[1]*b[2]-a[2]*b[1],a[3]*b[1]-a[0]*b[2]+a[1]*b[3]+a[2]*b[0],a[3]*b[2]+a[0]*b[1]-a[1]*b[0]+a[2]*b[3],a[3]*b[3]-a[0]*b[0]-a[1]*b[1]-a[2]*b[2]];}
export function axisAngle(axis:Vec3,angle:number):Quat{const u=unit(axis),s=Math.sin(angle/2);return [u[0]*s,u[1]*s,u[2]*s,Math.cos(angle/2)];}
export function rotate(q:Quat,p:Vec3):Vec3{const u:Vec3=[q[0],q[1],q[2]],v=cross(u,p);return add(p,add(scale(v,2*q[3]),scale(cross(u,v),2)));}
export function faceOrientation(normal:Vec3,tilt=0):Quat{const target:Vec3=[0,0,1],c=dot(normal,target);let q:Quat;if(c<-.999999){q=axisAngle(Math.abs(normal[0])<.9?cross(normal,[1,0,0]):cross(normal,[0,1,0]),Math.PI);}else{const a=cross(normal,target),w=1+c,length=Math.hypot(...a,w);q=[a[0]/length,a[1]/length,a[2]/length,w/length];}return multiply(axisAngle([0,0,1],tilt),q);}
export function slerp(a:Quat,b:Quat,t:number):Quat{let cosine=a.reduce((sum,v,i)=>sum+v*b[i],0);if(cosine<0){b=b.map(v=>-v) as Quat;cosine=-cosine;}if(cosine>.9995){const q=a.map((v,i)=>v+(b[i]-v)*t),length=Math.hypot(...q);return q.map(v=>v/length) as Quat;}const theta=Math.acos(Math.max(-1,Math.min(1,cosine))),sine=Math.sin(theta),x=Math.sin((1-t)*theta)/sine,y=Math.sin(t*theta)/sine;return a.map((v,i)=>v*x+b[i]*y) as Quat;}
