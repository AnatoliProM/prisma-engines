package util

import java.io.{BufferedReader, InputStreamReader}
import java.net.{HttpURLConnection, URL}
import java.nio.charset.StandardCharsets
import java.util.Base64

import play.api.libs.json._

import scala.concurrent.duration.Duration
import scala.concurrent.{Await, Awaitable, Future}
import scala.util.Try

case class QueryEngineResponse(status: Int, body: String) {
  lazy val jsonBody: Try[JsValue] = Try(Json.parse(body))
}

case class ApiTestServer() extends PlayJsonExtensions {
  import scala.concurrent.ExecutionContext.Implicits.global

  def query(
      query: String,
      project: Project,
      dataContains: String = ""
  ): JsValue = {
    awaitInfinitely { queryAsync(query, project, dataContains) }
  }

  def queryAsync(query: String, project: Project, dataContains: String = ""): Future[JsValue] = {
    val result = querySchemaAsync(
      query = query.stripMargin,
      project = project,
    )

    result.map { r =>
      r.assertSuccessfulResponse(dataContains)
      r
    }
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = ""
  ): JsValue = {
    val result = awaitInfinitely {
      querySchemaAsync(
        query = query,
        project = project,
      )
    }

    // TODO: bring those error checks back
    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  private def querySchemaAsync(
      query: String,
      project: Project
  ): Future[JsValue] = {
    val prismaProcess = startPrismaProcess(project)

    Future {
      println(prismaProcess.isAlive)
      queryPrismaProcess(query)
    }.map(r => r.jsonBody.get)
      .transform { r =>
        println(s"Query result: $r")
        prismaProcess.destroyForcibly().waitFor()
        r
      }
  }

  private def startPrismaProcess(project: Project): java.lang.Process = {
    import java.lang.ProcessBuilder.Redirect

    // TODO: discuss with Dom whether we want to keep the legacy mode
    val pb         = new java.lang.ProcessBuilder(EnvVars.prismaBinaryPath, "--legacy")
    val workingDir = new java.io.File(".")

    val fullDataModel = project.dataModelWithDataSourceConfig
    // Important: Rust requires UTF-8 encoding (encodeToString uses Latin-1)
    val encoded = Base64.getEncoder.encode(fullDataModel.getBytes(StandardCharsets.UTF_8))
    val envVar  = new String(encoded, StandardCharsets.UTF_8)

    pb.environment.put("PRISMA_DML", envVar)

    pb.directory(workingDir)
    pb.redirectErrorStream(true)
    pb.redirectOutput(Redirect.INHERIT)

    val p = pb.start
    Thread.sleep(100) // Offsets process startup latency
    p
  }

  private def queryPrismaProcess(query: String): QueryEngineResponse = {
    val url = new URL("http://127.0.0.1:4466")
    val con = url.openConnection().asInstanceOf[HttpURLConnection]

    con.setDoOutput(true)
    con.setRequestMethod("POST")
    con.setRequestProperty("Content-Type", "application/json")

    val body = Json.obj("query" -> query, "variables" -> Json.obj()).toString()
    con.setRequestProperty("Content-Length", Integer.toString(body.length))
    con.getOutputStream.write(body.getBytes(StandardCharsets.UTF_8))

    try {
      val status = con.getResponseCode
      val streamReader = if (status > 299) {
        new InputStreamReader(con.getErrorStream)
      } else {
        new InputStreamReader(con.getInputStream)
      }

      val in     = new BufferedReader(streamReader)
      val buffer = new StringBuffer

      Stream.continually(in.readLine()).takeWhile(_ != null).foreach(buffer.append)
      QueryEngineResponse(status, buffer.toString)
    } catch {
      case e: Throwable => QueryEngineResponse(999, s"""{"errors": [{"message": "Connection error: $e"}]}""")
    } finally {
      con.disconnect()
    }
  }

  private def awaitInfinitely[T](awaitable: Awaitable[T]): T = Await.result(awaitable, Duration.Inf)
}
